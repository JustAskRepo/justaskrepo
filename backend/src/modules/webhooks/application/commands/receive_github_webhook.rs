use std::sync::Arc;

use crate::{
    infrastructure::{AppContext, context::WebhooksContext},
    modules::installations::{
        RemoveInstallationCommand, SyncInstallationCommand, handle_remove_installation,
        handle_sync_installation,
    },
    modules::webhooks::api::{ReceiveGitHubWebhookCommand, WebhookResponse},
    shared_kernel::error::AppError,
};

use crate::modules::webhooks::domain::signature;
use crate::modules::webhooks::infrastructure::github_payloads::{self, InstallationChange};

/// Verify, understand, dispatch. In that order, and the order is the security
/// property: nothing about the payload is read until the signature over the raw
/// bytes checks out.
///
/// Calling another module's `api.rs` directly is what this module is *for*.
/// ADR-004 carves out exactly this exception — a dispatcher is not a domain
/// module, and routing through the event bus instead would mean publishing an
/// event about a state change that has not happened yet.
#[tracing::instrument(
    skip_all,
    fields(event = %cmd.event, delivery = %cmd.delivery_id),
    err
)]
pub(crate) async fn run(
    cmd: ReceiveGitHubWebhookCommand,
    webhooks: Arc<WebhooksContext>,
    ctx: &AppContext,
) -> Result<WebhookResponse, AppError> {
    signature::verify(&cmd.body, cmd.signature.as_deref(), &webhooks.secret)?;

    let Some(change) = github_payloads::parse(&cmd.event, &cmd.body)? else {
        return Ok(WebhookResponse::Ignored);
    };

    match change {
        InstallationChange::Sync(installation_id) => {
            let synced =
                handle_sync_installation(SyncInstallationCommand { installation_id }, ctx).await?;

            tracing::info!(
                installation_id = installation_id.0,
                repositories = synced.repositories,
                newly_installed = synced.newly_installed,
                suspended = synced.suspended,
                "installation synced from webhook"
            );
        }
        InstallationChange::Removed(installation_id) => {
            let removed =
                handle_remove_installation(RemoveInstallationCommand { installation_id }, ctx)
                    .await?;

            tracing::info!(
                installation_id = installation_id.0,
                affected_users = removed.affected_users,
                "installation removed from webhook"
            );
        }
    }

    Ok(WebhookResponse::Handled)
}
