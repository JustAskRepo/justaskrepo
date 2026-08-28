use sqlx::PgPool;

use crate::{
    modules::auth::domain::{github_profile::GitHubProfile, user::User},
    shared_kernel::{
        error::AppError,
        types::{GitHubId, UserId},
    },
};

pub(in crate::modules::auth) async fn upsert_user(
    profile: &GitHubProfile,
    db: &PgPool,
) -> Result<UserId, AppError> {
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO users (github_id, username, name, email, avatar_url, last_login_at)
        VALUES ($1, $2, $3, $4, $5, now())
        ON CONFLICT (github_id) DO UPDATE
        SET username      = EXCLUDED.username,
            name          = EXCLUDED.name,
            email         = EXCLUDED.email,
            avatar_url    = EXCLUDED.avatar_url,
            updated_at    = now(),
            last_login_at = now()
        RETURNING id
        "#,
        profile.github_user_id.0,
        profile.username,
        profile.name,
        profile.email,
        profile.avatar_url
    )
    .fetch_one(db)
    .await
    .map_err(AppError::internal)?;

    Ok(UserId(id))
}

/// `None` rather than an error when the row is gone: a session in Valkey can
/// outlive the user it names, and "no such user" is an answer, not a failure.
///
/// Only the columns the caller can use are selected. `created_at` and
/// `last_login_at` are audit fields — a read query that hauls them along
/// invites a response type to start carrying them.
#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn find_user_by_id(
    user_id: UserId,
    db: &PgPool,
) -> Result<Option<User>, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT id, github_id, username, name, email, avatar_url
        FROM users
        WHERE id = $1
        "#,
        user_id.0
    )
    .fetch_optional(db)
    .await
    .map_err(AppError::internal)?;

    Ok(row.map(|row| User {
        id: UserId(row.id),
        github_id: GitHubId(row.github_id),
        username: row.username,
        name: row.name,
        email: row.email,
        avatar_url: row.avatar_url,
    }))
}
