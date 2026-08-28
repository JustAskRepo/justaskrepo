use std::time::Duration;

use deadpool_redis::redis::{cmd, pipe};

use crate::modules::auth::domain::session::Session;
use crate::shared_kernel::{
    error::AppError,
    types::{SessionId, UserId},
};

fn session_key(session_id: &str) -> String {
    format!("session:{session_id}")
}

fn user_index_key(user_id: i64) -> String {
    format!("user_sessions:{user_id}")
}

#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn create_session(
    session_id: &SessionId,
    session: &Session,
    idle_ttl: Duration,
    absolute_ttl: Duration,
    valkey: deadpool_redis::Pool,
) -> Result<(), AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let record = serde_json::to_string(session).map_err(AppError::internal)?;
    let index_key = user_index_key(session.user_id.0);

    pipe()
        .atomic()
        .cmd("SET")
        .arg(session_key(&session_id.0))
        .arg(record)
        .arg("EX")
        .arg(idle_ttl.as_secs())
        .ignore()
        .cmd("SADD")
        .arg(&index_key)
        .arg(session_id.0.as_str())
        .ignore()
        .cmd("EXPIRE")
        .arg(&index_key)
        .arg(absolute_ttl.as_secs())
        .ignore()
        .query_async::<()>(&mut conn)
        .await
        .map_err(AppError::internal)?;

    Ok(())
}

#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn load_session(
    session_id: &SessionId,
    valkey: deadpool_redis::Pool,
) -> Result<Option<Session>, AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let record: Option<String> = cmd("GET")
        .arg(session_key(&session_id.0))
        .query_async(&mut conn)
        .await
        .map_err(AppError::internal)?;

    record
        .map(|record| serde_json::from_str(&record).map_err(AppError::internal))
        .transpose()
}

#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn touch_session(
    session_id: &SessionId,
    session: &Session,
    idle_ttl: Duration,
    valkey: deadpool_redis::Pool,
) -> Result<(), AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let record = serde_json::to_string(session).map_err(AppError::internal)?;

    cmd("SET")
        .arg(session_key(&session_id.0))
        .arg(record)
        .arg("EX")
        .arg(idle_ttl.as_secs())
        .query_async::<()>(&mut conn)
        .await
        .map_err(AppError::internal)?;

    Ok(())
}

#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn delete_session(
    session_id: &SessionId,
    user_id: UserId,
    valkey: deadpool_redis::Pool,
) -> Result<(), AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;

    pipe()
        .atomic()
        .cmd("DEL")
        .arg(session_key(&session_id.0))
        .ignore()
        .cmd("SREM")
        .arg(user_index_key(user_id.0))
        .arg(session_id.0.as_str())
        .ignore()
        .query_async::<()>(&mut conn)
        .await
        .map_err(AppError::internal)?;

    Ok(())
}

/// "Log out everywhere" — bounded by the per-user index rather than a scan of
/// the keyspace, which is the entire reason that index exists.
///
/// Reads the index, deletes what it names, then removes **exactly those ids**
/// from the index — `SREM` of what we read, not `DEL` of the whole key. A login
/// landing between the read and the write adds its id to the same set, and
/// dropping the key wholesale would unindex that brand-new session while
/// leaving its record alive: still usable, but invisible to the next bulk
/// revocation. Removing only what we saw leaves the newcomer indexed.
#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn delete_all_sessions(
    user_id: UserId,
    valkey: deadpool_redis::Pool,
) -> Result<usize, AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let index_key = user_index_key(user_id.0);

    let session_ids: Vec<String> = cmd("SMEMBERS")
        .arg(&index_key)
        .query_async(&mut conn)
        .await
        .map_err(AppError::internal)?;

    // `SREM key` with no members is an error, not a no-op.
    if session_ids.is_empty() {
        return Ok(0);
    }

    let mut batch = pipe();
    let batch = batch.atomic();
    for session_id in &session_ids {
        batch.cmd("DEL").arg(session_key(session_id)).ignore();
    }

    batch
        .cmd("SREM")
        .arg(&index_key)
        .arg(&session_ids)
        .ignore()
        .query_async::<()>(&mut conn)
        .await
        .map_err(AppError::internal)?;

    Ok(session_ids.len())
}
