// infrastructure/config.rs
//
// The ONLY file in src/ permitted to read environment variables. Architecture
// test rule 10 enforces that by text search — everything else receives what it
// needs through AppContext.
//
// Two rules shape this file:
//
//   1. Parse once, at startup. A missing DATABASE_URL kills the process at
//      second zero, not at the first request an hour into a deploy.
//   2. Secrets are SecretString, never String. Debug prints "[REDACTED]", so a
//      stray `tracing::info!(?config)` cannot leak the app private key.

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use secrecy::SecretString;

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    Missing(&'static str),

    #[error("invalid value for {var}: {message}")]
    Invalid { var: &'static str, message: String },

    #[error("invalid configuration: {0}")]
    Inconsistent(String),
}

impl ConfigError {
    fn invalid(var: &'static str, message: impl Into<String>) -> Self {
        Self::Invalid {
            var,
            message: message.into(),
        }
    }
}

// ─── Shape ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub valkey: ValkeyConfig,
    pub qdrant: QdrantConfig,
    pub gemini: GeminiConfig,
    pub github: GitHubAppConfig,
    pub session: SessionConfig,
}

#[derive(Debug)]
pub struct ServerConfig {
    /// Where Axum listens. Not the same as `public_url` — behind a proxy or in
    /// Docker these differ.
    pub bind_addr: SocketAddr,
    /// The origin the browser sees, no trailing slash. Used to build the OAuth
    /// redirect URI and to validate the `Origin` header on mutating requests.
    pub public_url: String,
    /// Directory holding the Next.js static export, served at `/`.
    pub static_dir: PathBuf,
}

#[derive(Debug)]
pub struct DatabaseConfig {
    /// Contains the password, hence secret.
    pub url: SecretString,
    pub max_connections: u32,
    pub acquire_timeout: Duration,
}

#[derive(Debug)]
pub struct ValkeyConfig {
    pub url: SecretString,
    pub pool_size: u32,
}

#[derive(Debug)]
pub struct QdrantConfig {
    pub url: String,
    /// Absent when running Qdrant locally without auth.
    pub api_key: Option<SecretString>,
    pub collection_prefix: String,
}

#[derive(Debug)]
pub struct GeminiConfig {
    pub api_key: SecretString,
    pub chat_model: String,
    pub embedding_model: String,
}

#[derive(Debug)]
pub struct GitHubAppConfig {
    pub app_id: u64,
    pub client_id: String,
    pub client_secret: SecretString,
    /// PEM contents, not a path — see `load_private_key`.
    pub private_key_pem: SecretString,
    pub webhook_secret: SecretString,
    /// Derived from `public_url`; not its own env var, so the two can never
    /// drift apart. The path is fixed by the router.
    pub redirect_uri: String,
}

#[derive(Debug)]
pub struct SessionConfig {
    /// `__Host-session` in production. Configurable because the `__Host-` prefix
    /// requires `Secure`, and dev over plain http may need a fallback —
    /// AUTHENTICATION.md §Session Cookie insists this be a config decision, not
    /// a `#[cfg(debug_assertions)]` one.
    pub cookie_name: String,
    /// Hard cap. A session dies this long after login regardless of activity.
    pub absolute_ttl: Duration,
    /// Sliding idle timeout, refreshed while the user is active.
    pub idle_ttl: Duration,
    /// Only refresh the idle TTL if `last_seen_at` is older than this. Without
    /// it, every authenticated request costs a Valkey write.
    pub refresh_threshold: Duration,
}

// ─── Loading ─────────────────────────────────────────────────────────────────

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let public_url = optional("PUBLIC_URL")
            .unwrap_or_else(|| "http://localhost:3001".to_owned())
            .trim_end_matches('/')
            .to_owned();

        let config = Self {
            server: ServerConfig {
                bind_addr: parsed_or("BIND_ADDR", "0.0.0.0:8080")?,
                public_url: public_url.clone(),
                static_dir: optional("STATIC_DIR")
                    .unwrap_or_else(|| "../frontend/out".to_owned())
                    .into(),
            },
            database: DatabaseConfig {
                url: secret("DATABASE_URL")?,
                max_connections: parsed_or("DATABASE_MAX_CONNECTIONS", "10")?,
                acquire_timeout: seconds_or("DATABASE_ACQUIRE_TIMEOUT_SECS", "10")?,
            },
            valkey: ValkeyConfig {
                url: secret("VALKEY_URL")?,
                pool_size: parsed_or("VALKEY_POOL_SIZE", "10")?,
            },
            qdrant: QdrantConfig {
                url: required("QDRANT_URL")?,
                api_key: optional("QDRANT_API_KEY").map(SecretString::from),
                collection_prefix: optional("QDRANT_COLLECTION_PREFIX")
                    .unwrap_or_else(|| "justaskrepo".to_owned()),
            },
            gemini: GeminiConfig {
                api_key: secret("GEMINI_API_KEY")?,
                chat_model: optional("GEMINI_CHAT_MODEL")
                    .unwrap_or_else(|| "gemini-2.5-flash".to_owned()),
                embedding_model: optional("GEMINI_EMBEDDING_MODEL")
                    .unwrap_or_else(|| "gemini-embedding-001".to_owned()),
            },
            github: GitHubAppConfig {
                app_id: parsed("GITHUB_APP_ID")?,
                client_id: required("GITHUB_CLIENT_ID")?,
                client_secret: secret("GITHUB_CLIENT_SECRET")?,
                private_key_pem: load_private_key()?,
                webhook_secret: secret("GITHUB_WEBHOOK_SECRET")?,
                redirect_uri: format!("{public_url}/api/auth/github/callback"),
            },
            session: SessionConfig {
                cookie_name: optional("SESSION_COOKIE_NAME")
                    .unwrap_or_else(|| "__Host-session".to_owned()),
                absolute_ttl: days_or("SESSION_ABSOLUTE_TTL_DAYS", "30")?,
                idle_ttl: days_or("SESSION_IDLE_TTL_DAYS", "7")?,
                refresh_threshold: seconds_or("SESSION_REFRESH_THRESHOLD_SECS", "300")?,
            },
        };

        config.validate()?;
        Ok(config)
    }

    /// Cross-field checks that no single `parse` can catch. These are the bugs
    /// that otherwise surface as "why did my session die after an hour".
    fn validate(&self) -> Result<(), ConfigError> {
        if self.session.idle_ttl > self.session.absolute_ttl {
            return Err(ConfigError::Inconsistent(
                "SESSION_IDLE_TTL_DAYS exceeds SESSION_ABSOLUTE_TTL_DAYS; the idle \
                 timeout would never fire"
                    .to_owned(),
            ));
        }

        if self.session.refresh_threshold >= self.session.idle_ttl {
            return Err(ConfigError::Inconsistent(
                "SESSION_REFRESH_THRESHOLD_SECS is not shorter than the idle TTL; \
                 sessions would expire before they are ever refreshed"
                    .to_owned(),
            ));
        }

        // `__Host-` cookies require Secure, which browsers only honour on https
        // or on localhost. Catch the mismatch here rather than as a silent
        // "login does nothing" in staging.
        let secure_origin = self.server.public_url.starts_with("https://")
            || self.server.public_url.starts_with("http://localhost")
            || self.server.public_url.starts_with("http://127.0.0.1");

        if self.session.cookie_name.starts_with("__Host-") && !secure_origin {
            return Err(ConfigError::Inconsistent(format!(
                "SESSION_COOKIE_NAME uses the __Host- prefix, which requires a secure \
                 origin, but PUBLIC_URL is {}. Use https, or set a plain cookie name \
                 for this environment.",
                self.server.public_url
            )));
        }

        Ok(())
    }
}

/// The App private key arrives one of two ways: base64 in an env var (prod,
/// where a PEM's newlines make a plain env var painful) or a file path (dev,
/// where GitHub hands you a `.pem` download).
fn load_private_key() -> Result<SecretString, ConfigError> {
    if let Some(encoded) = optional("GITHUB_APP_PRIVATE_KEY_B64") {
        let bytes = BASE64.decode(encoded.trim()).map_err(|e| {
            ConfigError::invalid(
                "GITHUB_APP_PRIVATE_KEY_B64",
                format!("not valid base64: {e}"),
            )
        })?;
        let pem = String::from_utf8(bytes).map_err(|_| {
            ConfigError::invalid("GITHUB_APP_PRIVATE_KEY_B64", "decoded bytes are not UTF-8")
        })?;
        return Ok(SecretString::from(pem));
    }

    if let Some(path) = optional("GITHUB_APP_PRIVATE_KEY_PATH") {
        let pem = std::fs::read_to_string(&path).map_err(|e| {
            ConfigError::invalid(
                "GITHUB_APP_PRIVATE_KEY_PATH",
                format!("cannot read {path}: {e}"),
            )
        })?;
        return Ok(SecretString::from(pem));
    }

    Err(ConfigError::Missing(
        "GITHUB_APP_PRIVATE_KEY_B64 or GITHUB_APP_PRIVATE_KEY_PATH",
    ))
}

// ─── Primitives ──────────────────────────────────────────────────────────────
//
// Every env read in the crate funnels through these four.

fn optional(key: &'static str) -> Option<String> {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

fn required(key: &'static str) -> Result<String, ConfigError> {
    optional(key).ok_or(ConfigError::Missing(key))
}

fn secret(key: &'static str) -> Result<SecretString, ConfigError> {
    required(key).map(SecretString::from)
}

fn parsed<T>(key: &'static str) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let raw = required(key)?;
    raw.parse()
        .map_err(|e| ConfigError::invalid(key, format!("{e} (got {raw:?})")))
}

fn parsed_or<T>(key: &'static str, default: &str) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let raw = optional(key).unwrap_or_else(|| default.to_owned());
    raw.parse()
        .map_err(|e| ConfigError::invalid(key, format!("{e} (got {raw:?})")))
}

fn seconds_or(key: &'static str, default: &str) -> Result<Duration, ConfigError> {
    parsed_or::<u64>(key, default).map(Duration::from_secs)
}

fn days_or(key: &'static str, default: &str) -> Result<Duration, ConfigError> {
    parsed_or::<u64>(key, default).map(|d| Duration::from_secs(d * 24 * 60 * 60))
}
