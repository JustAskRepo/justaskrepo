use std::sync::Arc;

use chrono::Utc;
use secrecy::ExposeSecret;
use sqlx::PgPool;

use crate::{
    infrastructure::context::AuthContext,
    modules::auth::api::CompleteGithubLoginCommand,
    shared_kernel::{error::AppError, types::SessionId},
};

use crate::modules::auth::domain::{random_token, session::Session};
use crate::modules::auth::infrastructure::{
    github_oauth_client, oauth_state_store, session_store, user_repository,
};

/// The session ID is minted fresh here, after the code exchange succeeds — never
/// reused from an inbound cookie. That is the session-fixation defense: a value
/// planted in the browser before login can never become an authenticated session.
#[tracing::instrument(skip_all, err)]
pub(crate) async fn run(
    cmd: CompleteGithubLoginCommand,
    auth: Arc<AuthContext>,
    valkey: deadpool_redis::Pool,
    http: reqwest::Client,
    db: PgPool,
) -> Result<SessionId, AppError> {
    oauth_state_store::verify_and_consume_state_valkey(cmd.state.expose_secret(), valkey.clone())
        .await?;
    let token =
        github_oauth_client::exchange_code_for_token(cmd.code.expose_secret(), &auth, &http)
            .await?;
    let profile = github_oauth_client::fetch_user_profile(&token, &http).await?;
    let user_id = user_repository::upsert_user(&profile, &db).await?;

    let session_id = SessionId(random_token::generate_token());
    let session = Session::new(
        user_id,
        profile.github_user_id,
        cmd.ip,
        cmd.user_agent,
        Utc::now(),
        auth.absolute_ttl,
    )?;
    session_store::create_session(
        &session_id,
        &session,
        auth.idle_ttl,
        auth.absolute_ttl,
        valkey,
    )
    .await?;

    tracing::info!(user_id = user_id.0, "session created");
    Ok(session_id)
}
