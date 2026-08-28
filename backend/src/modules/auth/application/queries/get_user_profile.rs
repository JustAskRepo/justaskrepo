use sqlx::PgPool;

use crate::{
    modules::auth::api::{GetUserProfileQuery, UserProfileResponse},
    shared_kernel::error::AppError,
};

use crate::modules::auth::infrastructure::user_repository;

#[tracing::instrument(skip_all, err)]
pub(crate) async fn run(
    query: GetUserProfileQuery,
    db: PgPool,
) -> Result<UserProfileResponse, AppError> {
    let user = user_repository::find_user_by_id(query.user_id, &db)
        .await?
        .ok_or(AppError::NotFound { resource: "user" })?;

    Ok(UserProfileResponse {
        user_id: user.id,
        github_id: user.github_id,
        username: user.username,
        name: user.name,
        email: user.email,
        avatar_url: user.avatar_url,
    })
}
