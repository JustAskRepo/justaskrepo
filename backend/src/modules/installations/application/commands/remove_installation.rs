use chrono::Utc;
use sqlx::PgPool;

use crate::{
    infrastructure::event_bus::EventBus,
    modules::installations::api::{RemoveInstallationCommand, RemoveInstallationResponse},
    shared_kernel::error::AppError,
};

use crate::modules::installations::domain::events::RepoUninstalledEvent;
use crate::modules::installations::infrastructure::{
    installation_repository, installation_token_cache,
};

/// The App was uninstalled. The grant goes, and everyone who held access
/// through it is announced so `auth` can end their sessions.
///
/// Deleting zero rows is success, not a failure: GitHub redelivers webhooks, and
/// a second `installation.deleted` for something already gone is the normal
/// case rather than an error worth a retry.
#[tracing::instrument(skip_all, fields(installation_id = cmd.installation_id.0), err)]
pub(crate) async fn run(
    cmd: RemoveInstallationCommand,
    valkey: deadpool_redis::Pool,
    db: PgPool,
    events: EventBus,
) -> Result<RemoveInstallationResponse, AppError> {
    // Before the delete — the foreign keys cascade these away, and the event
    // cannot name a user whose row is already gone.
    let user_ids = installation_repository::linked_user_ids(cmd.installation_id, &db).await?;

    installation_repository::delete_installation(cmd.installation_id, &db).await?;
    installation_token_cache::forget(cmd.installation_id, &valkey).await;

    let now = Utc::now();
    for user_id in &user_ids {
        events
            .publish(RepoUninstalledEvent::new(
                cmd.installation_id,
                *user_id,
                now,
            ))
            .await;
    }

    Ok(RemoveInstallationResponse {
        affected_users: user_ids.len(),
    })
}
