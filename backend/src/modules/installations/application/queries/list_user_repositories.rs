use sqlx::PgPool;

use crate::{
    modules::installations::api::{ListUserRepositoriesQuery, UserRepositoryResponse},
    shared_kernel::error::AppError,
};

use crate::modules::installations::infrastructure::installation_repository;

/// Identity and access only. Index state belongs to `indexing`, and the route
/// handler is what combines the two (ARCHITECTURE.md §5.1) — this module must
/// not learn that indexing exists.
#[tracing::instrument(skip_all, err)]
pub(crate) async fn run(
    query: ListUserRepositoriesQuery,
    db: PgPool,
) -> Result<Vec<UserRepositoryResponse>, AppError> {
    installation_repository::list_user_repositories(query.user_id, &db).await
}
