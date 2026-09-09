use std::sync::Arc;

use crate::{
    infrastructure::context::InstallationsContext,
    modules::installations::api::{GetInstallationTokenQuery, InstallationTokenResponse},
    shared_kernel::error::AppError,
};

use crate::modules::installations::infrastructure::{github_app_client, installation_token_cache};

/// Cache, then mint. The cache is checked first because GitHub rate-limits the
/// mint and a token is good for an hour; it is never *trusted* first, because
/// `installation_token_cache` only ever returns one with real life left in it.
#[tracing::instrument(skip_all, fields(installation_id = query.installation_id.0), err)]
pub(crate) async fn run(
    query: GetInstallationTokenQuery,
    installations: Arc<InstallationsContext>,
    valkey: deadpool_redis::Pool,
    http: reqwest::Client,
) -> Result<InstallationTokenResponse, AppError> {
    if let Some(cached) = installation_token_cache::get(query.installation_id, &valkey).await {
        return Ok(InstallationTokenResponse {
            token: cached.token,
            expires_at: cached.expires_at,
        });
    }

    let minted = github_app_client::mint_installation_token(
        query.installation_id,
        installations.app_id,
        &installations.private_key_pem,
        &http,
    )
    .await?;

    installation_token_cache::put(query.installation_id, &minted, &valkey).await;

    Ok(InstallationTokenResponse {
        token: minted.token,
        expires_at: minted.expires_at,
    })
}
