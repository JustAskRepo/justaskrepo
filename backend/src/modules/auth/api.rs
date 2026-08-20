// <module_name>/api.rs
//
// This is the ONLY public surface of this module.
// All Commands, Queries, and their handler functions live here.
// Handlers are thin — they delegate to application/ use cases.

use crate::modules::auth::application::commands::{complete_github_login, start_github_login};
use crate::{infrastructure::AppContext, shared_kernel::error::AppError};

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
    pub state: String,
    pub code: String,
}

#[derive(Debug)]
pub struct CompleteGithubLoginResponse {
    pub redirect_url: String,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_complete_github_login(
    cmd: CompleteGithubLoginCommand,
    ctx: &AppContext,
) -> Result<CompleteGithubLoginResponse, AppError> {
    let redirect_url =
        complete_github_login::run(cmd, ctx.auth.clone(), ctx.valkey.clone(), ctx.http.clone())
            .await?;

    Ok(CompleteGithubLoginResponse { redirect_url })
}
// ─── Queries ─────────────────────────────────────────────────────────────────

// Read-only intent to <describe read operation>. No side effects.
// #[derive(Debug)]
// pub struct GetExampleQuery {
//     // pub field: Type,
// }

// #[derive(Debug)]
// pub struct ExampleResponse {
//     // pub field: Type,
// }

// #[tracing::instrument(skip(ctx))]
// pub async fn handle_get_example(
//     query: GetExampleQuery,
//     ctx: &AppContext,
// ) -> Result<ExampleResponse, AppError> {
//     todo!("Delegate to application/queries/get_example.rs")
// }
