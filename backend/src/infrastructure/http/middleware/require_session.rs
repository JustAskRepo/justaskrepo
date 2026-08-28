use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method, header::ORIGIN, request::Parts},
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

/// What the session layer proved about this request, handed to every protected
/// handler.
///
/// `Clone`, not `Copy`: `SessionId` owns a `String`, and `Copy` means "a
/// duplicate is just a bitwise copy of these bytes" — which is false for
/// anything owning a heap allocation, since two structs would end up pointing
/// at one buffer and both try to free it. Rust will not let you derive `Copy`
/// here at all.
///
/// The session id belongs here rather than being re-read from the cookie in
/// each handler: the middleware already parsed and validated it, and a route
/// that re-derives auth state is a second place for that logic to drift.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub github_id: GitHubId,
}

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Self>().cloned().ok_or_else(|| {
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
    verify_origin(request.method(), request.headers(), &ctx.public_origin)?;

    let session_id = jar
        .get(&ctx.auth.cookie_name)
        .map(|cookie| SessionId(cookie.value().to_owned()))
        .ok_or(AppError::Unauthorized)?;

    // Cloned because building the Query *moves* the id into it — the original
    // binding would be gone by the next line. Rust has no implicit copy for
    // owned values; you either hand it over or duplicate it deliberately.
    let session = handle_get_session(
        GetSessionQuery {
            session_id: session_id.clone(),
        },
        &ctx,
    )
    .await?;

    request.extensions_mut().insert(CurrentUser {
        session_id,
        user_id: session.user_id,
        github_id: session.github_id,
    });

    Ok(next.run(request).await)
}

fn verify_origin(method: &Method, headers: &HeaderMap, expected: &str) -> Result<(), AppError> {
    if method.is_safe() {
        return Ok(());
    }

    let origin = headers
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Forbidden)?;

    if !origin.eq_ignore_ascii_case(expected) {
        tracing::warn!(%method, %origin, "rejected a cross-origin write on a session route");
        return Err(AppError::Forbidden);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    const OURS: &str = "http://localhost:3001";

    fn headers(origin: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(value) = origin.and_then(|origin| HeaderValue::from_str(origin).ok()) {
            headers.insert(ORIGIN, value);
        }
        headers
    }

    fn is_forbidden(result: Result<(), AppError>) -> bool {
        matches!(result, Err(AppError::Forbidden))
    }

    /// Reads never mutate, and `Origin` is absent from plenty of legitimate
    /// ones — a demand for it here would break normal navigation.
    #[test]
    fn safe_methods_never_need_an_origin() {
        assert!(verify_origin(&Method::GET, &headers(None), OURS).is_ok());
        assert!(verify_origin(&Method::HEAD, &headers(None), OURS).is_ok());
    }

    #[test]
    fn a_write_from_our_own_page_is_allowed() {
        assert!(verify_origin(&Method::POST, &headers(Some(OURS)), OURS).is_ok());
    }

    #[test]
    fn a_write_from_another_site_is_rejected() {
        assert!(is_forbidden(verify_origin(
            &Method::POST,
            &headers(Some("https://evil.example")),
            OURS,
        )));
    }

    /// The comparison is equality, not a prefix: `localhost:3001.evil.test` is
    /// a different site that happens to start with our origin.
    #[test]
    fn a_lookalike_origin_is_rejected() {
        assert!(is_forbidden(verify_origin(
            &Method::POST,
            &headers(Some("http://localhost:3001.evil.test")),
            OURS,
        )));
    }

    /// The same host over plain http vs https is two origins, and only one of
    /// them is us.
    #[test]
    fn a_scheme_mismatch_is_rejected() {
        assert!(is_forbidden(verify_origin(
            &Method::POST,
            &headers(Some("https://localhost:3001")),
            OURS,
        )));
    }

    /// curl, a stripped header, or a browser that declined to send one.
    #[test]
    fn a_write_with_no_origin_at_all_is_rejected() {
        assert!(is_forbidden(verify_origin(
            &Method::DELETE,
            &headers(None),
            OURS,
        )));
    }
}
