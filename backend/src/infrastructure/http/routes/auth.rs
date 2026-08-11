use axum::{Router, extract::State, response::Redirect, routing::get};
// use serde::Serialize;

use crate::{
    infrastructure::AppContext,
    modules::auth::{StartGithubLoginCommand, handle_start_github_login},
    shared_kernel::error::AppError,
};

pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/auth/github", get(github_auth))
        .route("/auth/github/callback", get(github_auth_callback))
}

async fn github_auth(State(ctx): State<AppContext>) -> Result<Redirect, AppError> {
    let res = handle_start_github_login(StartGithubLoginCommand, &ctx).await?;
    Ok(Redirect::temporary(&res.authorize_url))
}

async fn github_auth_callback(State(_ctx): State<AppContext>) {}
