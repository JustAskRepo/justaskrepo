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
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use super::{
    config::{AppConfig, RateLimitConfig},
    rate_limiter::RateLimitPolicy,
};
use secrecy::SecretString;
use sqlx::PgPool;

#[derive(Debug)]
pub struct AuthContext {
    pub cookie_name: String,
    pub absolute_ttl: Duration,
    pub idle_ttl: Duration,
    pub refresh_threshold: Duration,
    pub client_id: String,
    pub client_secret: SecretString,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Copy)]
pub struct RateLimits {
    pub auth: RateLimitPolicy,
}

#[derive(Clone)]
pub struct AppContext {
    pub auth: Arc<AuthContext>,
    pub public_origin: Arc<str>,
    pub rate_limits: RateLimits,
    pub trusted_proxy_hops: u8,
    pub db: PgPool,
    pub valkey: deadpool_redis::Pool,
    pub http: reqwest::Client,
    pub started_at: Instant,
    // TODO(event_bus): infrastructure/event_bus.rs, per ADR-004.
}

impl AppContext {
    pub async fn new(config: &AppConfig) -> anyhow::Result<Self> {
        let db = super::db::connect_db(&config.database).await?;
        let valkey = super::valkey::connect_valkey(&config.valkey).await?;
        let auth = Arc::new(AuthContext::from(config));
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(3))
            .user_agent("justaskrepo/0.1")
            .build()?;
        Ok(Self {
            auth,
            public_origin: Arc::from(config.server.public_origin.as_str()),
            rate_limits: RateLimits::from(&config.rate_limit),
            trusted_proxy_hops: config.server.trusted_proxy_hops,
            db,
            valkey,
            http,
            started_at: Instant::now(),
        })
    }
}
impl From<&RateLimitConfig> for RateLimits {
    fn from(config: &RateLimitConfig) -> Self {
        Self {
            auth: RateLimitPolicy {
                name: "auth",
                max_requests: config.auth_max_requests,
                window: config.auth_window,
            },
        }
    }
}

impl From<&AppConfig> for AuthContext {
    fn from(config: &AppConfig) -> Self {
        Self {
            cookie_name: config.session.cookie_name.clone(),
            absolute_ttl: config.session.absolute_ttl,
            idle_ttl: config.session.idle_ttl,
            refresh_threshold: config.session.refresh_threshold,
            client_id: config.github.client_id.clone(),
            client_secret: config.github.client_secret.clone(),
            redirect_uri: config.github.redirect_uri.clone(),
        }
    }
}
