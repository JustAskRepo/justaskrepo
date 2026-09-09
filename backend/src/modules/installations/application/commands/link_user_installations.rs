use std::sync::Arc;

use secrecy::SecretString;
use sqlx::PgPool;

use crate::{
    infrastructure::{context::InstallationsContext, event_bus::EventBus},
    modules::installations::api::{
        LinkUserInstallationsCommand, LinkUserInstallationsResponse, RemoveInstallationCommand,
    },
    shared_kernel::{error::AppError, types::InstallationId},
};

use crate::modules::installations::application::commands::remove_installation;
use crate::modules::installations::domain::installation::{Installation, Repository};
use crate::modules::installations::infrastructure::{
    github_app_client, github_user_client, installation_repository,
};

/// Spends the user access token, once, on the only question it can answer:
/// which installations this human may see, and which repositories within them
/// (ADR-009 decision 1). The token is never stored and never leaves this call.
#[tracing::instrument(skip_all, err)]
pub(crate) async fn run(
    cmd: LinkUserInstallationsCommand,
    context: Arc<InstallationsContext>,
    valkey: deadpool_redis::Pool,
    http: reqwest::Client,
    db: PgPool,
    events: EventBus,
) -> Result<LinkUserInstallationsResponse, AppError> {
    // Read before the rewrite, so the dropped links can be worked out after it.
    let previous = installation_repository::user_installation_ids(cmd.user_id, &db).await?;

    let installations = github_user_client::list_user_installations(&cmd.user_token, &http).await?;

    let repositories = fetch_repositories(&installations, &cmd.user_token, &http).await?;

    installation_repository::replace_user_access(cmd.user_id, &installations, &repositories, &db)
        .await?;

    let dropped: Vec<InstallationId> = previous
        .into_iter()
        .filter(|id| !installations.iter().any(|i| i.id.0 == *id))
        .map(InstallationId)
        .collect();

    if !dropped.is_empty() {
        remove_the_dead(&dropped, &context, &valkey, &http, &db, &events).await;
    }

    Ok(LinkUserInstallationsResponse {
        installations: installations.len(),
        repositories: repositories.len(),
    })
}

/// A link disappearing has two very different causes and only GitHub can tell
/// them apart: either the installation is gone, or it still exists and this user
/// lost access to it — removed from the organization that owns it, say.
///
/// So every dropped id is checked, and **only a definite 404 deletes anything.**
/// A transient failure leaves the row alone: deleting a live installation
/// because GitHub was briefly unreachable would revoke real access and cascade
/// away real data.
///
/// This is the recovery path for a missed `installation.deleted` webhook, and it
/// recovers the whole thing rather than just the row — `RemoveInstallationCommand`
/// publishes `RepoUninstalledEvent` for every user still linked, so their
/// sessions end here exactly as the webhook would have ended them. That part
/// matters: ARCHITECTURE.md §6 notes the in-memory bus can lose that event, and
/// until the outbox exists this is the only thing that catches it.
///
/// Never fatal. The login that triggered it has already succeeded, and
/// housekeeping must not undo that.
async fn remove_the_dead(
    dropped: &[InstallationId],
    context: &Arc<InstallationsContext>,
    valkey: &deadpool_redis::Pool,
    http: &reqwest::Client,
    db: &PgPool,
    events: &EventBus,
) {
    for &installation_id in dropped {
        match github_app_client::fetch_installation(
            installation_id,
            context.app_id,
            &context.private_key_pem,
            http,
        )
        .await
        {
            // Still there — this user lost access, the grant did not end.
            Ok(_) => tracing::debug!(
                installation_id = installation_id.0,
                "user lost access to an installation that still exists"
            ),

            Err(AppError::NotFound { .. }) => {
                tracing::info!(
                    installation_id = installation_id.0,
                    "installation is gone from GitHub; removing it (missed webhook)"
                );

                if let Err(error) = remove_installation::run(
                    RemoveInstallationCommand { installation_id },
                    valkey.clone(),
                    db.clone(),
                    events.clone(),
                )
                .await
                {
                    tracing::warn!(%error, installation_id = installation_id.0, "removal failed");
                }
            }

            // Unreachable, rate-limited, refused: say nothing, delete nothing.
            Err(error) => tracing::warn!(
                %error,
                installation_id = installation_id.0,
                "could not confirm whether an installation still exists; leaving it"
            ),
        }
    }
}

/// Any failure here aborts the whole reconciliation rather than storing what it
/// managed to collect. `replace_user_access` deletes before it inserts, so a
/// half-read set would revoke repositories the user still has — a transient
/// GitHub error must not cost someone their access.
async fn fetch_repositories(
    installations: &[Installation],
    token: &SecretString,
    http: &reqwest::Client,
) -> Result<Vec<Repository>, AppError> {
    let mut out = Vec::new();

    for installation in installations {
        // A suspended installation grants nothing and GitHub will refuse the
        // request. Its row is still written, so an unsuspend has something to
        // come back to.
        if installation.suspended_at.is_some() {
            continue;
        }

        out.extend(github_user_client::list_user_repositories(installation.id, token, http).await?);
    }

    Ok(out)
}
