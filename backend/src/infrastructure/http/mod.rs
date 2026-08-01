// infrastructure/http/mod.rs
//
// The HTTP composition layer. This is the only place in the crate that knows
// both axum and every module's api.rs. No module knows about either.
//
// Delete this directory and every module still compiles. That is the test for
// whether something belongs in it.

pub mod error;
pub mod middleware;
pub mod routes;

use axum::Router;

use super::AppContext;

/// Assembles the whole HTTP surface.
///
/// Everything is nested under `/api` and that prefix is load-bearing: `/` serves
/// the Next.js static export, and the dev proxy only forwards `/api/*` to Axum.
/// A route registered outside `/api` is unreachable in development.
pub fn router(ctx: AppContext) -> Router {
    // Public — no session required.
    let public = Router::new().merge(routes::health::routes());

    // Authenticated — every route below the session layer.
    //
    // TODO(auth): the moment the auth module lands, this becomes:
    //
    //   let protected = Router::new()
    //       .route("/me", get(routes::auth::me))
    //       .route("/repositories", get(routes::repositories::list))
    //       .route_layer(from_fn_with_state(ctx.clone(), middleware::require_session));
    //
    // `route_layer`, not `layer`: it runs only on routes that actually matched,
    // so an unknown path 404s instead of 401ing.

    Router::new().nest("/api", public).with_state(ctx)
    // TODO(static): .fallback_service(ServeDir::new(&config.server.static_dir))
    //   to serve the Next.js export at `/`. Needs tower-http; in dev the Next
    //   dev server serves the UI instead, which is why compose points
    //   STATIC_DIR at an unused path.
    //
    // NOTE: no CorsLayer, deliberately. The UI and API share one origin, so
    //   there is nothing to permit — adding CORS would only grant access that
    //   does not currently exist (AUTHENTICATION.md §Same-Origin Model).
}
