use axum::{
    Router,
    extract::{Query, State},
    response::Redirect,
    routing::get,
};
use serde::Deserialize;

use crate::{
    infrastructure::AppContext,
    modules::auth::{
        CompleteGithubLoginCommand, StartGithubLoginCommand, handle_complete_github_login,
        handle_start_github_login,
    },
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

#[derive(Debug, Deserialize)]
struct GithubCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn github_auth_callback(
    State(ctx): State<AppContext>,
    Query(query): Query<GithubCallbackQuery>,
) -> Result<Redirect, AppError> {
    if let Some(error) = query.error {
        tracing::warn!(%error, "github denied the oauth callback");
        return Err(AppError::Unauthorized);
    }
    let (Some(code), Some(state)) = (query.code, query.state) else {
        return Err(AppError::Validation(
            "callback missing code or state".into(),
        ));
    };

    let res =
        handle_complete_github_login(CompleteGithubLoginCommand { code, state }, &ctx).await?;
    Ok(Redirect::temporary(&res.redirect_url))
}
