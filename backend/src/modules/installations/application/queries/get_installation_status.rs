use sqlx::PgPool;

use crate::{
    modules::installations::api::{GetInstallationStatusQuery, InstallationStatusResponse},
    shared_kernel::error::AppError,
};

use crate::modules::installations::infrastructure::installation_repository;

/// Answers the one question the login callback asks: send this user to the
/// dashboard, or to the install screen (ARCHITECTURE.md §5.1).
#[tracing::instrument(skip_all, err)]
pub(crate) async fn run(
    query: GetInstallationStatusQuery,
    db: PgPool,
) -> Result<InstallationStatusResponse, AppError> {
    let has_any = installation_repository::user_has_usable_installation(query.user_id, &db).await?;

    Ok(InstallationStatusResponse { has_any })
}
