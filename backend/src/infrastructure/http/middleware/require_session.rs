use axum::{
    extract::{Request, State},
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;

use crate::{
    infrastructure::AppContext,
    modules::auth::{GetSessionQuery, handle_get_session},
    shared_kernel::{
        error::AppError,
        types::{GitHubId, SessionId, UserId},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct CurrentUser {
    pub user_id: UserId,
    pub github_id: GitHubId,
}

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().copied().ok_or_else(|| {
            AppError::internal(anyhow::anyhow!(
                "CurrentUser extracted on a route with no require_session layer"
            ))
        })
    }
}

pub async fn require_session(
    State(ctx): State<AppContext>,
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let session_id = jar
        .get(&ctx.auth.cookie_name)
        .map(|cookie| SessionId(cookie.value().to_owned()))
        .ok_or(AppError::Unauthorized)?;

    let session = handle_get_session(GetSessionQuery { session_id }, &ctx).await?;

    request.extensions_mut().insert(CurrentUser {
        user_id: session.user_id,
        github_id: session.github_id,
    });

    Ok(next.run(request).await)
}
