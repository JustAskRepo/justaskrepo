use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;

use crate::{
    infrastructure::{context::InstallationsContext, event_bus::EventBus},
    modules::installations::api::{
        GetInstallationTokenQuery, SyncInstallationCommand, SyncInstallationResponse,
    },
    shared_kernel::error::AppError,
};

use crate::modules::installations::application::queries::get_installation_token;
use crate::modules::installations::domain::events::RepoInstalledEvent;
use crate::modules::installations::infrastructure::{github_app_client, installation_repository};

/// Re-reads an installation from GitHub and replaces what we stored for it.
///
/// One command covers every `installation.*` and `installation_repositories.*`
/// action except deletion, because after re-fetching they all mean the same
/// thing: this grant changed, here is what it is now. Distinguishing "created"
/// from "repositories added" would only matter if we trusted the payload, and
/// §8 of the build guide is emphatic that we should not.
#[tracing::instrument(skip_all, fields(installation_id = cmd.installation_id.0), err)]
pub(crate) async fn run(
    cmd: SyncInstallationCommand,
    installations: Arc<InstallationsContext>,
    valkey: deadpool_redis::Pool,
    http: reqwest::Client,
    db: PgPool,
    events: EventBus,
) -> Result<SyncInstallationResponse, AppError> {
    let installation = github_app_client::fetch_installation(
        cmd.installation_id,
        installations.app_id,
        &installations.private_key_pem,
        &http,
    )
    .await?;

    // Suspended: the record is updated, the repository set is left alone.
    // GitHub will not mint a token for a suspended installation anyway, and
    // clearing the set would leave a later unsuspend with nothing to restore.
    if installation.suspended_at.is_some() {
        installation_repository::save_installation(&installation, &db).await?;
        tracing::info!("installation is suspended; repositories left untouched");
        return Ok(SyncInstallationResponse {
            repositories: 0,
            newly_installed: 0,
            suspended: true,
        });
    }

    let token = get_installation_token::run(
        GetInstallationTokenQuery {
            installation_id: cmd.installation_id,
        },
        installations,
        valkey,
        http.clone(),
    )
    .await?;

    let repositories =
        github_app_client::list_installation_repositories(cmd.installation_id, &token.token, &http)
            .await?;

    // Read before the replace, so "new" means new to us rather than merely
    // present. Announcing the whole set on every webhook would ask `indexing`
    // to re-index repositories that have not changed.
    let known =
        installation_repository::installation_repository_ids(cmd.installation_id, &db).await?;

    installation_repository::replace_installation(&installation, &repositories, &db).await?;

    let now = Utc::now();
    let mut newly_installed = 0;
    for repo in &repositories {
        if !known.contains(&repo.id.0) {
            newly_installed += 1;
            events
                .publish(RepoInstalledEvent::new(
                    cmd.installation_id,
                    repo.id,
                    repo.full_name.clone(),
                    now,
                ))
                .await;
        }
    }

    Ok(SyncInstallationResponse {
        repositories: repositories.len(),
        newly_installed,
        suspended: false,
    })
}
