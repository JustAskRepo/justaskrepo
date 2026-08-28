use sqlx::PgPool;

use crate::{
    modules::auth::domain::github_profile::GitHubProfile,
    shared_kernel::{error::AppError, types::UserId},
};

pub(in crate::modules::auth) async fn upsert_user(
    profile: &GitHubProfile,
    db: &PgPool,
) -> Result<UserId, AppError> {
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO users (github_id, username, name, email, avatar_url, last_login_at)
        VALUES ($1, $2, $3, $4, $5, now())
        ON CONFLICT (github_id) DO UPDATE
        SET username      = EXCLUDED.username,
            name          = EXCLUDED.name,
            email         = EXCLUDED.email,
            avatar_url    = EXCLUDED.avatar_url,
            updated_at    = now(),
            last_login_at = now()
        RETURNING id
        "#,
        profile.github_user_id.0,
        profile.username,
        profile.name,
        profile.email,
        profile.avatar_url
    )
    .fetch_one(db)
    .await
    .map_err(AppError::internal)?;

    Ok(UserId(id))
}
