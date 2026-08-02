// infrastructure/context.rs
//
// AppContext — the dependency container handed to every Command/Query handler.
//
// It lives here rather than in shared_kernel/ because it holds a sqlx pool, and
// architecture rule 9 keeps framework types out of shared_kernel. That is the
// right split: shared_kernel is what a module would carry with it if it were
// extracted into its own service; a connection pool is what it would leave
// behind and rebuild.
//
// Clone is cheap by construction — every field is an Arc or an already-shared
// handle. That is what lets axum hold it as `State<AppContext>` in both route
// handlers and middleware.
use std::time::Instant;

use sqlx::PgPool;

use super::config::AppConfig;

#[derive(Clone)]
pub struct AppContext {
    pub db: PgPool,
    pub started_at: Instant,
    // TODO(valkey): sessions, oauth state, rate limits. Blocked on picking a
    //   client crate — see AUTHENTICATION.md §Data Model.
    // TODO(event_bus): infrastructure/event_bus.rs, per ADR-004.
}

impl AppContext {
    /// Builds every long-lived resource the process needs. Fails loudly if a
    /// dependency is unreachable — a backend that starts without its database
    /// only moves the outage to the first user request.
    pub async fn new(config: &AppConfig) -> anyhow::Result<Self> {
        let db = super::db::connect_db(&config.database).await?;
        Ok(Self {
            db,
            started_at: Instant::now(),
        })
    }
}
