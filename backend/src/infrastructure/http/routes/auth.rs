use std::net::SocketAddr;

use axum::{
    Router,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, header},
    response::Redirect,
    routing::get,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;

use axum::Json;

use crate::{
    infrastructure::{AppContext, context::AuthContext, http::middleware::CurrentUser},
    modules::auth::{
        CompleteGithubLoginCommand, StartGithubLoginCommand, handle_complete_github_login,
        handle_start_github_login,
    },
    shared_kernel::{
        error::AppError,
        types::{GitHubId, SessionId, UserId},
    },
};

/// Attacker-controlled and unbounded; capped before it reaches Valkey.
const MAX_USER_AGENT: usize = 256;

pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/auth/github", get(github_auth))
        .route("/auth/github/callback", get(github_auth_callback))
}

async fn github_auth(State(ctx): State<AppContext>) -> Result<Redirect, AppError> {
    let res = handle_start_github_login(StartGithubLoginCommand, &ctx).await?;
    Ok(Redirect::temporary(&res.authorize_url))
}

#[derive(Debug, serde::Serialize)]
pub struct MeResponse {
    user_id: UserId,
    github_id: GitHubId,
}

pub async fn me(user: CurrentUser) -> Json<MeResponse> {
    Json(MeResponse {
        user_id: user.user_id,
        github_id: user.github_id,
    })
}

#[derive(Debug, Deserialize)]
struct GithubCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn github_auth_callback(
    State(ctx): State<AppContext>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    jar: CookieJar,
    headers: HeaderMap,
    Query(query): Query<GithubCallbackQuery>,
) -> Result<(CookieJar, Redirect), AppError> {
    if let Some(error) = query.error {
        tracing::warn!(%error, "github denied the oauth callback");
        return Err(AppError::Unauthorized);
    }
    let (Some(code), Some(state)) = (query.code, query.state) else {
        return Err(AppError::Validation(
            "callback missing code or state".into(),
        ));
    };

    let res = handle_complete_github_login(
        CompleteGithubLoginCommand {
            code: code.into(),
            state: state.into(),
            ip: client_ip(&headers, peer),
            user_agent: user_agent(&headers),
        },
        &ctx,
    )
    .await?;

    Ok((
        jar.add(session_cookie(&ctx.auth, res.session_id)?),
        Redirect::temporary("/dashboard/"),
    ))
}

/// The `__Host-` prefix the default cookie name carries is only honoured when the
/// cookie is `Secure`, `Path=/`, and carries no `Domain` — so those three are not
/// independent choices here, and dropping `secure` in dev would silently disable
/// the prefix rather than just relax transport. The dev fallback is a different
/// cookie *name* via `SESSION_COOKIE_NAME`, never a weakened attribute set.
fn session_cookie(auth: &AuthContext, session_id: SessionId) -> Result<Cookie<'static>, AppError> {
    let max_age = time::Duration::try_from(auth.absolute_ttl).map_err(AppError::internal)?;

    Ok(Cookie::build((auth.cookie_name.clone(), session_id.0))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(max_age)
        .build())
}

/// Prefers `X-Forwarded-For` because the dev setup proxies `/api/*` through the
/// Next dev server, where the peer address is always the proxy. The header is
/// client-spoofable, which is acceptable only because the value is recorded for
/// audit and never trusted for an authorization decision. If a route ever needs
/// a trustworthy IP, this must become a trusted-proxy-aware parse first.
fn client_ip(headers: &HeaderMap, peer: SocketAddr) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(|| peer.ip().to_string(), str::to_owned)
}

fn user_agent(headers: &HeaderMap) -> String {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .chars()
        .take(MAX_USER_AGENT)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn session_cookie_satisfies_host_prefix_rules() {
        let auth = AuthContext {
            cookie_name: "__Host-session".to_owned(),
            absolute_ttl: Duration::from_secs(2_592_000),
            idle_ttl: Duration::from_secs(604_800),
            refresh_threshold: Duration::from_secs(300),
            client_id: "id".to_owned(),
            client_secret: "secret".into(),
            redirect_uri: "http://localhost:8080/api/auth/github/callback".to_owned(),
        };

        let rendered = session_cookie(&auth, SessionId("abc123".to_owned()))
            .map(|c| c.to_string())
            .unwrap_or_default();

        println!("Set-Cookie: {rendered}");
        assert!(rendered.starts_with("__Host-session=abc123"));
        assert!(rendered.contains("HttpOnly"));
        assert!(rendered.contains("Secure"));
        assert!(rendered.contains("SameSite=Lax"));
        assert!(rendered.contains("Path=/"));
        assert!(rendered.contains("Max-Age=2592000"));
        assert!(
            !rendered.contains("Domain"),
            "__Host- forbids a Domain attribute"
        );
    }
}
