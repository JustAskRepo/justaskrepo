use crate::shared_kernel::error::AppError;
use deadpool_redis::redis::cmd;

pub(in crate::modules::auth) async fn store_state_valkey(
    state: &str,
    valkey: deadpool_redis::Pool,
) -> Result<(), AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let ttl = std::time::Duration::from_secs(10 * 60); // 10 minutes
    cmd("SET")
        .arg(format!("oauth_state:{state}"))
        .arg(1)
        .arg("EX")
        .arg(ttl.as_secs())
        .query_async::<()>(&mut conn)
        .await
        .map_err(AppError::internal)?;
    Ok(())
}

pub(in crate::modules::auth) async fn verify_and_consume_state_valkey(
    state: &str,
    valkey: deadpool_redis::Pool,
) -> Result<(), AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let result: Option<String> = cmd("GETDEL")
        .arg(format!("oauth_state:{state}"))
        .query_async(&mut conn)
        .await
        .map_err(AppError::internal)?;

    if result.is_none() {
        tracing::warn!("oauth callback rejected: state missing or expired");
        return Err(AppError::Unauthorized);
    }
    Ok(())
}
