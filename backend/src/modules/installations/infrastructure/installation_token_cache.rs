// modules/installations/infrastructure/installation_token_cache.rs
//
// Valkey, not Postgres and not in-process (ADR-009 decision 5). Postgres would
// mean an hour-long bearer credential at rest for no gain; in-process would be
// the assumption that breaks the day `indexing` moves out, and `indexing` is
// the heaviest consumer of these.

use chrono::{TimeDelta, Utc};
use deadpool_redis::redis::cmd;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::modules::installations::domain::installation::InstallationToken;
use crate::shared_kernel::{error::AppError, types::InstallationId};

/// How long before real expiry the cache stops serving a token.
///
/// A token handed out with four minutes left is a job that fails partway. The
/// margin means every token a caller receives has at least this long to live,
/// which is what makes "mint once, use for a while" safe.
const SAFETY_MARGIN: TimeDelta = TimeDelta::minutes(5);

fn token_key(installation_id: InstallationId) -> String {
    format!("installation_token:{}", installation_id.0)
}

/// The stored shape. Exposing the secret into a plain `String` happens here and
/// only here — `InstallationToken` deliberately refuses to serialize itself, so
/// writing one out has to be a decision someone made on purpose.
#[derive(Serialize, Deserialize)]
struct CachedToken {
    token: String,
    expires_at: chrono::DateTime<Utc>,
}

/// A miss and a failure are the same answer: mint a new one.
///
/// This is the second deliberate exception to ADR-008's fail-closed stance on
/// Valkey, alongside the rate limiter. A cache is not an authority — refusing to
/// mint because the cache is unreachable would turn a slow path into an outage,
/// and the credential is obtainable from GitHub regardless.
#[tracing::instrument(skip_all, fields(installation_id = installation_id.0))]
pub(in crate::modules::installations) async fn get(
    installation_id: InstallationId,
    valkey: &deadpool_redis::Pool,
) -> Option<InstallationToken> {
    match read(installation_id, valkey).await {
        Ok(hit) => hit,
        Err(error) => {
            tracing::warn!(%error, "token cache unreadable — minting a fresh token");
            None
        }
    }
}

async fn read(
    installation_id: InstallationId,
    valkey: &deadpool_redis::Pool,
) -> Result<Option<InstallationToken>, AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let record: Option<String> = cmd("GET")
        .arg(token_key(installation_id))
        .query_async(&mut conn)
        .await
        .map_err(AppError::internal)?;

    let Some(record) = record else {
        return Ok(None);
    };

    let cached: CachedToken = serde_json::from_str(&record).map_err(AppError::internal)?;
    Ok(Some(InstallationToken {
        token: SecretString::from(cached.token),
        expires_at: cached.expires_at,
    }))
}

/// Stores a token under a TTL that expires it `SAFETY_MARGIN` early, so the key
/// is gone before the credential is too stale to hand out. A token already
/// inside the margin is not cached at all rather than cached for a moment.
///
/// Failure is logged and swallowed for the same reason as `get`.
#[tracing::instrument(skip_all, fields(installation_id = installation_id.0))]
pub(in crate::modules::installations) async fn put(
    installation_id: InstallationId,
    token: &InstallationToken,
    valkey: &deadpool_redis::Pool,
) {
    let ttl = (token.expires_at - SAFETY_MARGIN - Utc::now()).num_seconds();
    if ttl <= 0 {
        tracing::warn!(
            expires_at = %token.expires_at,
            "github returned a token inside the safety margin; not caching it"
        );
        return;
    }

    if let Err(error) = write(installation_id, token, ttl, valkey).await {
        tracing::warn!(%error, "could not cache the installation token");
    }
}

async fn write(
    installation_id: InstallationId,
    token: &InstallationToken,
    ttl_secs: i64,
    valkey: &deadpool_redis::Pool,
) -> Result<(), AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let record = serde_json::to_string(&CachedToken {
        token: token.token.expose_secret().to_owned(),
        expires_at: token.expires_at,
    })
    .map_err(AppError::internal)?;

    cmd("SET")
        .arg(token_key(installation_id))
        .arg(record)
        .arg("EX")
        .arg(ttl_secs)
        .query_async::<()>(&mut conn)
        .await
        .map_err(AppError::internal)
}

/// Drops a cached token — used when the installation it belongs to is gone.
///
/// The credential is already useless to GitHub, but leaving it in Valkey for
/// the rest of its hour is a secret outliving the thing it was issued for.
#[tracing::instrument(skip_all, fields(installation_id = installation_id.0))]
pub(in crate::modules::installations) async fn forget(
    installation_id: InstallationId,
    valkey: &deadpool_redis::Pool,
) {
    let result = async {
        let mut conn = valkey.get().await.map_err(AppError::internal)?;
        cmd("DEL")
            .arg(token_key(installation_id))
            .query_async::<()>(&mut conn)
            .await
            .map_err(AppError::internal)
    }
    .await;

    if let Err(error) = result {
        tracing::warn!(%error, "could not drop the cached installation token");
    }
}
