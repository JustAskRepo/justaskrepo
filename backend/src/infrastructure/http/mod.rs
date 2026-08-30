// infrastructure/http/mod.rs
//
// The HTTP composition layer. This is the only place in the crate that knows
// both axum and every module's api.rs. No module knows about either.
//
// Delete this directory and every module still compiles. That is the test for
// whether something belongs in it.

pub mod client_ip;
pub mod error;
pub mod middleware;
pub mod routes;

use axum::{Router, middleware::from_fn_with_state};

use super::AppContext;

/// Assembles the whole HTTP surface.
///
/// Everything is nested under `/api` and that prefix is load-bearing: `/` serves
/// the Next.js static export, and the dev proxy only forwards `/api/*` to Axum.
/// A route registered outside `/api` is unreachable in development.
pub fn router(ctx: AppContext) -> Router {
    let auth = routes::auth::routes().route_layer(from_fn_with_state(
        middleware::RateLimitState::new(ctx.clone(), ctx.rate_limits.auth),
        middleware::rate_limit,
    ));

    // Public — no session required.
    let public = Router::new().merge(routes::health::routes()).merge(auth);

    // Authenticated — every route below the session layer.
    //
    // `route_layer`, not `layer`: it runs only on routes that actually matched,
    // so an unknown path 404s instead of 401ing.
    let protected = routes::auth::protected_routes()
        .route_layer(from_fn_with_state(ctx.clone(), middleware::require_session));

    Router::new()
        .nest("/api", public.merge(protected))
        .with_state(ctx)
    // TODO(static): .fallback_service(ServeDir::new(&config.server.static_dir))
    //   to serve the Next.js export at `/`. Needs tower-http; in dev the Next
    //   dev server serves the UI instead, which is why compose points
    //   STATIC_DIR at an unused path.
    //
    // NOTE: no CorsLayer, deliberately. The UI and API share one origin, so
    //   there is nothing to permit — adding CORS would only grant access that
    //   does not currently exist (AUTHENTICATION.md §Same-Origin Model).
}
