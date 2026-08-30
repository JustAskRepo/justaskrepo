use std::net::SocketAddr;

use axum::{
    Router,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::Redirect,
    routing::{get, post},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;

use axum::Json;

use crate::{
    infrastructure::{
        AppContext,
        context::AuthContext,
        http::{client_ip::client_ip, middleware::CurrentUser},
    },
    modules::auth::{
        CompleteGithubLoginCommand, GetUserProfileQuery, RevokeAllSessionsCommand,
        RevokeSessionCommand, StartGithubLoginCommand, handle_complete_github_login,
        handle_get_user_profile, handle_revoke_all_sessions, handle_revoke_session,
        handle_start_github_login,
    },
    shared_kernel::{
        error::AppError,
        types::{GitHubId, SessionId, UserId},
    },
};

/// Attacker-controlled and unbounded; capped before it reaches Valkey.
const MAX_USER_AGENT: usize = 256;

/// Public — reachable without a session. Both are the login handshake itself,
/// which by definition runs before there is anything to authenticate.
pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/auth/github", get(github_auth))
        .route("/auth/github/callback", get(github_auth_callback))
}

/// Mounted behind `require_session` in `http::router`. Kept here rather than
/// listed route-by-route in the composition root so that adding an
/// authenticated auth route touches one file, and so the public/protected split
/// is visible in the file that owns the handlers.
pub fn protected_routes() -> Router<AppContext> {
    Router::new()
        .route("/me", get(me))
        .route("/auth/logout", post(logout))
        .route("/auth/logout/all", post(logout_all))
}

async fn github_auth(State(ctx): State<AppContext>) -> Result<Redirect, AppError> {
    let res = handle_start_github_login(StartGithubLoginCommand, &ctx).await?;
    Ok(Redirect::temporary(&res.authorize_url))
}

#[derive(Debug, serde::Serialize)]
pub struct MeResponse {
    user_id: UserId,
    github_id: GitHubId,
    username: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

pub async fn me(
    State(ctx): State<AppContext>,
    user: CurrentUser,
) -> Result<Json<MeResponse>, AppError> {
    let profile = handle_get_user_profile(
        GetUserProfileQuery {
            user_id: user.user_id,
        },
        &ctx,
    )
    .await?;

    Ok(Json(MeResponse {
        user_id: profile.user_id,
        github_id: profile.github_id,
        username: profile.username,
        name: profile.name,
        avatar_url: profile.avatar_url,
    }))
}

/// Revoking server-side is only half a logout. A browser still holding a cookie
/// for a key that no longer exists keeps sending it and keeps collecting 401s;
/// the `Max-Age=0` cookie is what actually clears the client.
///
/// `204`, not `200`: there is no body worth sending, and the frontend's
/// `logout()` deliberately does not read one.
pub async fn logout(
    State(ctx): State<AppContext>,
    jar: CookieJar,
    user: CurrentUser,
) -> Result<(CookieJar, StatusCode), AppError> {
    handle_revoke_session(
        RevokeSessionCommand {
            session_id: user.session_id,
            user_id: user.user_id,
        },
        &ctx,
    )
    .await?;

    Ok((jar.add(removal_cookie(&ctx.auth)), StatusCode::NO_CONTENT))
}

/// "Log out everywhere", this device included — the caller's own session is in
/// the index like any other, so no special case is needed to catch it.
pub async fn logout_all(
    State(ctx): State<AppContext>,
    jar: CookieJar,
    user: CurrentUser,
) -> Result<(CookieJar, StatusCode), AppError> {
    handle_revoke_all_sessions(
        RevokeAllSessionsCommand {
            user_id: user.user_id,
        },
        &ctx,
    )
    .await?;

    Ok((jar.add(removal_cookie(&ctx.auth)), StatusCode::NO_CONTENT))
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
            ip: client_ip(&headers, peer, ctx.trusted_proxy_hops).to_string(),
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

/// A browser matches a deletion against name + path + domain, so this has to
/// reproduce what `session_cookie` set or the original quietly survives. The
/// `__Host-` prefix tightens that further: a removal missing `Secure` or
/// `Path=/` is rejected outright, leaving the user still signed in.
fn removal_cookie(auth: &AuthContext) -> Cookie<'static> {
    Cookie::build((auth.cookie_name.clone(), ""))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::ZERO)
        .build()
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

    fn auth_context() -> AuthContext {
        AuthContext {
            cookie_name: "__Host-session".to_owned(),
            absolute_ttl: Duration::from_secs(2_592_000),
            idle_ttl: Duration::from_secs(604_800),
            refresh_threshold: Duration::from_secs(300),
            client_id: "id".to_owned(),
            client_secret: "secret".into(),
            redirect_uri: "http://localhost:8080/api/auth/github/callback".to_owned(),
        }
    }

    #[test]
    fn session_cookie_satisfies_host_prefix_rules() {
        let auth = auth_context();

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

    /// A removal only deletes a cookie the browser considers *the same* cookie.
    /// Every attribute here is load-bearing, and getting one wrong fails in the
    /// worst possible way: a 204 that leaves the user signed in.
    #[test]
    fn removal_cookie_can_actually_delete_the_session_cookie() {
        let rendered = removal_cookie(&auth_context()).to_string();

        println!("Set-Cookie: {rendered}");
        assert!(rendered.starts_with("__Host-session="));
        assert!(rendered.contains("Max-Age=0"));
        assert!(rendered.contains("HttpOnly"));
        assert!(rendered.contains("Secure"));
        assert!(rendered.contains("Path=/"));
        assert!(
            !rendered.contains("Domain"),
            "__Host- forbids a Domain attribute"
        );
    }
}
