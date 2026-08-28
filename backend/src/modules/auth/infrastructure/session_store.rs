use std::time::Duration;

use deadpool_redis::redis::{cmd, pipe};

use crate::modules::auth::domain::session::Session;
use crate::shared_kernel::{
    error::AppError,
    types::{SessionId, UserId},
};

fn session_key(session_id: &SessionId) -> String {
    format!("session:{}", session_id.0)
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
        .arg(session_key(session_id))
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
        .arg(session_key(session_id))
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
        .arg(session_key(session_id))
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
        .arg(session_key(session_id))
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
