use std::sync::Arc;

use crate::{
    infrastructure::context::InstallationsContext,
    modules::installations::api::{GetInstallationUrlQuery, InstallationUrlResponse},
    shared_kernel::error::AppError,
};

use crate::modules::installations::infrastructure::github_app_client;

/// Not `async`: this reads config and formats a string. Every other use case in
/// the codebase awaits something, and this one has nothing to await — making it
/// async would only promise I/O that never happens.
pub(crate) fn run(
    _query: GetInstallationUrlQuery,
    installations: Arc<InstallationsContext>,
) -> Result<InstallationUrlResponse, AppError> {
    Ok(InstallationUrlResponse {
        install_url: github_app_client::build_install_url(&installations.app_slug),
    })
}
