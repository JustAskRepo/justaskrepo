use anyhow::Context;
use deadpool_redis::{Config, PoolConfig, Runtime, redis::cmd};
use secrecy::ExposeSecret;

pub async fn connect_valkey(
    valkey_config: &super::config::ValkeyConfig,
) -> anyhow::Result<deadpool_redis::Pool> {
    let mut cfg = Config::from_url(valkey_config.url.expose_secret());
    cfg.pool = Some(PoolConfig::new(valkey_config.pool_size as usize));
    let pool = cfg
        .create_pool(Some(Runtime::Tokio1))
        .context("building Valkey connection pool (check VALKEY_URL format)")?;
    let mut conn = pool
        .get()
        .await
        .context("connecting to Valkey — is it reachable at VALKEY_URL?")?;
    cmd("PING")
        .query_async::<()>(&mut conn)
        .await
        .context("Valkey PING failed after connecting")?;
    Ok(pool)
}
