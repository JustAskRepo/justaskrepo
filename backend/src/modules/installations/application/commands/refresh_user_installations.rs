use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    infrastructure::{context::InstallationsContext, event_bus::EventBus},
    modules::installations::api::{
        RefreshUserInstallationsCommand, RefreshUserInstallationsResponse, SyncInstallationCommand,
    },
    shared_kernel::{error::AppError, types::InstallationId},
};

use crate::modules::installations::application::commands::sync_installation;
use crate::modules::installations::infrastructure::{
    installation_repository, installation_sync_throttle,
};

/// Brings this user's grants up to date before something reads them.
///
/// It exists because webhook delivery is not a freshness guarantee. A webhook
/// can be missed, delayed, or — in development — never arrive at all, and the
/// dashboard being wrong about which repositories someone may see is not a
/// cosmetic failure. This makes the read path responsible for its own accuracy
/// rather than trusting a notification it cannot verify arrived.
///
/// Every installation is gated by `installation_sync_throttle`, so a polling
/// dashboard costs one GitHub request a minute per installation rather than one
/// per poll.
#[tracing::instrument(skip_all, fields(user_id = cmd.user_id.0, force = cmd.force), err)]
pub(crate) async fn run(
    cmd: RefreshUserInstallationsCommand,
    context: Arc<InstallationsContext>,
    valkey: deadpool_redis::Pool,
    http: reqwest::Client,
    db: PgPool,
    events: EventBus,
) -> Result<RefreshUserInstallationsResponse, AppError> {
    let installation_ids = installation_repository::user_installation_ids(cmd.user_id, &db).await?;

    let mut synced = 0;
    let mut skipped = 0;

    for installation_id in installation_ids.into_iter().map(InstallationId) {
        // A person pressing Refresh is not a poll, and throttling them is
        // throttling the one caller who already knows the data is stale. The
        // window is still reset so the polling that follows stays cheap.
        if cmd.force {
            installation_sync_throttle::renew(installation_id, &valkey).await;
        } else if !installation_sync_throttle::claim(installation_id, &valkey).await {
            skipped += 1;
            continue;
        }

        // Per-installation failures are logged and stepped over. This runs to
        // make a read more accurate; it must never be the reason the read
        // fails, because the stored answer is still the best one we have.
        match sync_installation::run(
            SyncInstallationCommand { installation_id },
            context.clone(),
            valkey.clone(),
            http.clone(),
            db.clone(),
            events.clone(),
        )
        .await
        {
            Ok(_) => synced += 1,
            Err(error) => tracing::warn!(
                %error,
                installation_id = installation_id.0,
                "could not refresh an installation; serving stored access"
            ),
        }
    }

    Ok(RefreshUserInstallationsResponse { synced, skipped })
}
