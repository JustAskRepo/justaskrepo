// <module_name>/api.rs
//
// This is the ONLY public surface of this module.
// All Commands, Queries, and their handler functions live here.
// Handlers are thin — they delegate to application/ use cases.

use secrecy::SecretString;

use crate::modules::auth::application::commands::{complete_github_login, start_github_login};
use crate::modules::auth::application::queries::get_session;
use crate::{
    infrastructure::AppContext,
    shared_kernel::{
        error::AppError,
        types::{GitHubId, SessionId, UserId},
    },
};

// ─── Commands ────────────────────────────────────────────────────────────────

// handle_start_github_login─────────────────────────────────────────────────────────────────
#[derive(Debug)]
pub struct StartGithubLoginCommand;

#[derive(Debug)]
pub struct StartGithubLoginResponse {
    pub authorize_url: String,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_start_github_login(
    cmd: StartGithubLoginCommand,
    ctx: &AppContext,
) -> Result<StartGithubLoginResponse, AppError> {
    let authorize_url = start_github_login::run(cmd, ctx.auth.clone(), ctx.valkey.clone()).await?;
    Ok(StartGithubLoginResponse { authorize_url })
}

// handle_complete_github_login─────────────────────────────────────────────────────────────────
#[derive(Debug)]
pub struct CompleteGithubLoginCommand {
    pub state: SecretString,
    pub code: SecretString,
    pub ip: String,
    pub user_agent: String,
}

#[derive(Debug)]
pub struct CompleteGithubLoginResponse {
    pub session_id: SessionId,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_complete_github_login(
    cmd: CompleteGithubLoginCommand,
    ctx: &AppContext,
) -> Result<CompleteGithubLoginResponse, AppError> {
    let session_id = complete_github_login::run(
        cmd,
        ctx.auth.clone(),
        ctx.valkey.clone(),
        ctx.http.clone(),
        ctx.db.clone(),
    )
    .await?;

    Ok(CompleteGithubLoginResponse { session_id })
}
// ─── Queries ─────────────────────────────────────────────────────────────────

// handle_get_session──────────────────────────────────────────────────────────
#[derive(Debug)]
pub struct GetSessionQuery {
    pub session_id: SessionId,
}

#[derive(Debug)]
pub struct SessionResponse {
    pub user_id: UserId,
    pub github_id: GitHubId,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_get_session(
    query: GetSessionQuery,
    ctx: &AppContext,
) -> Result<SessionResponse, AppError> {
    get_session::run(query, ctx.auth.clone(), ctx.valkey.clone()).await
}
