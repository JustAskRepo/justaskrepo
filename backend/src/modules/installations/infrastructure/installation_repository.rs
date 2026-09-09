use sqlx::PgPool;

use crate::modules::installations::api::UserRepositoryResponse;
use crate::modules::installations::domain::installation::{Installation, Repository};
use crate::shared_kernel::{
    error::AppError,
    types::{InstallationId, RepoFullName, RepositoryId, UserId},
};

/// Replaces everything login reconciliation owns for one user, in one
/// transaction (ADR-009 decision 1).
///
/// Replace rather than diff, for the reason the webhook path will need too: a
/// set computed by applying deltas is wrong the moment one is missed or arrives
/// out of order, and there is no way to notice. Rewriting the set cannot drift.
///
/// The caller must have fetched *all* of it successfully. A partial set here
/// would delete access the user still has, which is worse than leaving the
/// previous, complete-but-stale answer in place.
#[tracing::instrument(skip_all, err)]
pub(in crate::modules::installations) async fn replace_user_access(
    user_id: UserId,
    installations: &[Installation],
    repositories: &[Repository],
    db: &PgPool,
) -> Result<(), AppError> {
    let mut tx = db.begin().await.map_err(AppError::internal)?;

    for installation in installations {
        upsert_installation(installation, &mut tx).await?;
    }

    let installation_ids: Vec<i64> = installations.iter().map(|i| i.id.0).collect();

    // Access this user no longer has stops existing. Without this the reconcile
    // only ever adds, and a revoked grant would outlive every future login.
    // An empty array deletes every link, which is the correct reading of
    // "GitHub says this user can see nothing".
    sqlx::query!(
        "DELETE FROM user_installations WHERE user_id = $1 AND installation_id <> ALL($2)",
        user_id.0,
        &installation_ids
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::internal)?;

    for installation_id in &installation_ids {
        sqlx::query!(
            r#"
            INSERT INTO user_installations (user_id, installation_id)
            VALUES ($1, $2)
            ON CONFLICT (user_id, installation_id) DO UPDATE SET linked_at = now()
            "#,
            user_id.0,
            installation_id
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::internal)?;
    }

    sqlx::query!(
        "DELETE FROM user_repositories WHERE user_id = $1",
        user_id.0
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::internal)?;

    for repo in repositories {
        sqlx::query!(
            r#"
            INSERT INTO user_repositories (
                user_id, repository_id, installation_id, full_name, private, default_branch
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id, repository_id) DO UPDATE
            SET installation_id = EXCLUDED.installation_id,
                full_name       = EXCLUDED.full_name,
                private         = EXCLUDED.private,
                default_branch  = EXCLUDED.default_branch,
                synced_at       = now()
            "#,
            user_id.0,
            repo.id.0,
            repo.installation_id.0,
            repo.full_name.0,
            repo.private,
            repo.default_branch,
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::internal)?;
    }

    tx.commit().await.map_err(AppError::internal)
}

/// Whether this user has any installation that currently grants anything.
///
/// Suspension is filtered here rather than deleted on receipt: a suspended
/// installation keeps its rows so an unsuspend has something to restore, and
/// every read is responsible for ignoring it.
#[tracing::instrument(skip_all, err)]
pub(in crate::modules::installations) async fn user_has_usable_installation(
    user_id: UserId,
    db: &PgPool,
) -> Result<bool, AppError> {
    let has_any = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM user_installations ui
            JOIN installations i ON i.id = ui.installation_id
            WHERE ui.user_id = $1 AND i.suspended_at IS NULL
        ) AS "has_any!"
        "#,
        user_id.0
    )
    .fetch_one(db)
    .await
    .map_err(AppError::internal)?;

    Ok(has_any)
}

/// The repositories this user may see, joined against their grants so a
/// suspended installation contributes nothing.
///
/// Reads `user_repositories`, never `installation_repositories`: the second is
/// what the *app* may reach, and for an organization that is a wider set than
/// any one member is entitled to (migration 0003).
///
/// Ordered by name so the dashboard has a stable list without sorting one.
#[tracing::instrument(skip_all, err)]
pub(in crate::modules::installations) async fn list_user_repositories(
    user_id: UserId,
    db: &PgPool,
) -> Result<Vec<UserRepositoryResponse>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT ur.repository_id, ur.installation_id, ur.full_name,
               ur.private, ur.default_branch
        FROM user_repositories ur
        JOIN installations i ON i.id = ur.installation_id
        WHERE ur.user_id = $1 AND i.suspended_at IS NULL
        ORDER BY ur.full_name
        "#,
        user_id.0
    )
    .fetch_all(db)
    .await
    .map_err(AppError::internal)?;

    Ok(rows
        .into_iter()
        .map(|row| {
            // GitHub guarantees `full_name` is exactly `owner/name`, and neither
            // half may contain a slash — so this split is lossless and the two
            // parts need no columns of their own.
            let (owner_login, name) = row
                .full_name
                .split_once('/')
                .unwrap_or(("", row.full_name.as_str()));

            UserRepositoryResponse {
                repository_id: RepositoryId(row.repository_id),
                installation_id: InstallationId(row.installation_id),
                owner_login: owner_login.to_owned(),
                name: name.to_owned(),
                full_name: RepoFullName(row.full_name.clone()),
                is_private: row.private,
                default_branch: row.default_branch,
            }
        })
        .collect())
}

/// The installation row itself, upserted on GitHub's own id.
///
/// Both writers land here — login reconciliation and webhook sync — and neither
/// may assume it ran first. That is the whole reason the primary key is
/// GitHub's id rather than one of ours (ADR-009 decision 3).
pub(in crate::modules::installations) async fn upsert_installation(
    installation: &Installation,
    conn: &mut sqlx::PgConnection,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        INSERT INTO installations (
            id, account_id, account_login, account_type,
            repository_selection, suspended_at
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (id) DO UPDATE
        SET account_id           = EXCLUDED.account_id,
            account_login        = EXCLUDED.account_login,
            account_type         = EXCLUDED.account_type,
            repository_selection = EXCLUDED.repository_selection,
            suspended_at         = EXCLUDED.suspended_at,
            updated_at           = now()
        "#,
        installation.id.0,
        installation.account_id,
        installation.account_login,
        installation.account_type,
        installation.repository_selection.as_str(),
        installation.suspended_at,
    )
    .execute(conn)
    .await
    .map_err(AppError::internal)?;

    Ok(())
}

/// Upserts the installation and nothing else. Used when it is suspended: the
/// grant has stopped working but its repository set must survive, or a later
/// unsuspend has nothing to restore (guide §8 — suspension is not deletion).
#[tracing::instrument(skip_all, fields(installation_id = installation.id.0), err)]
pub(in crate::modules::installations) async fn save_installation(
    installation: &Installation,
    db: &PgPool,
) -> Result<(), AppError> {
    let mut conn = db.acquire().await.map_err(AppError::internal)?;
    upsert_installation(installation, &mut conn).await
}

/// Which repositories we currently believe this installation covers.
///
/// Read before a replace so the caller can tell which are genuinely new and
/// publish `RepoInstalledEvent` for those alone — re-announcing the whole set on
/// every webhook would give `indexing` a reason to re-index everything.
pub(in crate::modules::installations) async fn installation_repository_ids(
    installation_id: InstallationId,
    db: &PgPool,
) -> Result<Vec<i64>, AppError> {
    sqlx::query_scalar!(
        "SELECT repository_id FROM installation_repositories WHERE installation_id = $1",
        installation_id.0
    )
    .fetch_all(db)
    .await
    .map_err(AppError::internal)
}

/// Installation plus its repository set, replaced together.
///
/// Replace, never diff: webhook delivery is at-least-once and unordered, so a
/// `.removed` can overtake the `.added` it followed. State rebuilt from a fresh
/// read of GitHub cannot drift; state accumulated from deltas silently can.
#[tracing::instrument(skip_all, fields(installation_id = installation.id.0), err)]
pub(in crate::modules::installations) async fn replace_installation(
    installation: &Installation,
    repositories: &[Repository],
    db: &PgPool,
) -> Result<(), AppError> {
    let mut tx = db.begin().await.map_err(AppError::internal)?;

    upsert_installation(installation, &mut tx).await?;

    sqlx::query!(
        "DELETE FROM installation_repositories WHERE installation_id = $1",
        installation.id.0
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::internal)?;

    for repo in repositories {
        sqlx::query!(
            r#"
            INSERT INTO installation_repositories (
                installation_id, repository_id, full_name, private, default_branch
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (installation_id, repository_id) DO UPDATE
            SET full_name      = EXCLUDED.full_name,
                private        = EXCLUDED.private,
                default_branch = EXCLUDED.default_branch
            "#,
            installation.id.0,
            repo.id.0,
            repo.full_name.0,
            repo.private,
            repo.default_branch,
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::internal)?;
    }

    // Grant the owner of a *personal* installation access to its repositories.
    //
    // The general rule is that installation-scoped data cannot answer a
    // per-user question, and that holds — except where the installation's
    // account *is* the user. A personal installation covers only repositories
    // owned by that account, and the owner reaches all of them, so here the
    // installation's set is exactly their set and no user token is needed to
    // say so. The WHERE clause is that argument, written down: the account type
    // must be `User`, and the row is created only for the user whose
    // `github_id` matches the account.
    //
    // An organization installation matches nothing here, correctly — whether a
    // given member sees a repository is a question only
    // `GET /user/installations/{id}/repositories` answers, so those additions
    // still wait for that member's next login (ADR-009 decision 4).
    for repo in repositories {
        sqlx::query!(
            r#"
            INSERT INTO user_repositories (
                user_id, repository_id, installation_id, full_name, private, default_branch
            )
            SELECT u.id, $2, $1, $3, $4, $5
            FROM installations i
            JOIN users u ON u.github_id = i.account_id
            WHERE i.id = $1 AND i.account_type = 'User'
            ON CONFLICT (user_id, repository_id) DO UPDATE
            SET installation_id = EXCLUDED.installation_id,
                full_name       = EXCLUDED.full_name,
                private         = EXCLUDED.private,
                default_branch  = EXCLUDED.default_branch,
                synced_at       = now()
            "#,
            installation.id.0,
            repo.id.0,
            repo.full_name.0,
            repo.private,
            repo.default_branch,
        )
        .execute(&mut *tx)
        .await
        .map_err(AppError::internal)?;
    }

    // Propagate removals into per-user access, in the same transaction.
    //
    // This is the one direction that can cross from installation-scoped data to
    // user-scoped data without a user access token, and it is worth being precise
    // about why: the installation's set is an *upper bound* on what any member can
    // reach through it. A repository that has left the installation is reachable
    // by nobody through it, so deleting those rows needs no per-user question.
    //
    // The reverse does not hold. A repository newly added to the installation is
    // not thereby visible to a given member — that answer lives behind
    // `GET /user/installations/{id}/repositories` and a token we do not have here
    // — so additions still wait for that user's next login (ADR-009 decision 4).
    //
    // Asymmetric on purpose: a missing repository is an inconvenience, while one
    // that lingers after being removed is an over-grant.
    let live_ids: Vec<i64> = repositories.iter().map(|r| r.id.0).collect();
    let pruned = sqlx::query!(
        "DELETE FROM user_repositories
         WHERE installation_id = $1 AND repository_id <> ALL($2)",
        installation.id.0,
        &live_ids
    )
    .execute(&mut *tx)
    .await
    .map_err(AppError::internal)?
    .rows_affected();

    tx.commit().await.map_err(AppError::internal)?;

    if pruned > 0 {
        tracing::info!(
            installation_id = installation.id.0,
            pruned,
            "revoked user access to repositories no longer in the installation"
        );
    }

    Ok(())
}

/// Whose access this installation carries — read *before* deleting it, because
/// the delete cascades these rows away and the event needs the ids.
pub(in crate::modules::installations) async fn linked_user_ids(
    installation_id: InstallationId,
    db: &PgPool,
) -> Result<Vec<UserId>, AppError> {
    let ids = sqlx::query_scalar!(
        "SELECT user_id FROM user_installations WHERE installation_id = $1",
        installation_id.0
    )
    .fetch_all(db)
    .await
    .map_err(AppError::internal)?;

    Ok(ids.into_iter().map(UserId).collect())
}

/// Drops the grant. Foreign keys carry away the repository set, the user links,
/// and every user's access rows with it.
///
/// A hard delete rather than a tombstone: an uninstalled App has no residual
/// meaning, and a reinstall arrives with a brand-new installation id anyway.
#[tracing::instrument(skip_all, fields(installation_id = installation_id.0), err)]
pub(in crate::modules::installations) async fn delete_installation(
    installation_id: InstallationId,
    db: &PgPool,
) -> Result<(), AppError> {
    sqlx::query!("DELETE FROM installations WHERE id = $1", installation_id.0)
        .execute(db)
        .await
        .map_err(AppError::internal)?;

    Ok(())
}

/// Which installations this user is linked to right now.
///
/// Read *before* `replace_user_access` rewrites them, so the caller can tell
/// which links the rewrite dropped. A dropped link is the only hint we ever get
/// that an installation may be gone — GitHub does not tell us twice.
pub(in crate::modules::installations) async fn user_installation_ids(
    user_id: UserId,
    db: &PgPool,
) -> Result<Vec<i64>, AppError> {
    sqlx::query_scalar!(
        "SELECT installation_id FROM user_installations WHERE user_id = $1",
        user_id.0
    )
    .fetch_all(db)
    .await
    .map_err(AppError::internal)
}
