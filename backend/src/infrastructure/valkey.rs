use deadpool_redis::{Config, PoolConfig, Runtime, redis::cmd};
use secrecy::ExposeSecret;

pub async fn connect_valkey(
    valkey_config: &super::config::ValkeyConfig,
) -> anyhow::Result<deadpool_redis::Pool> {
    let mut cfg = Config::from_url(valkey_config.url.expose_secret());
    cfg.pool = Some(PoolConfig::new(valkey_config.pool_size as usize));
    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
    let mut conn = pool.get().await?;
    cmd("PING").query_async::<()>(&mut conn).await?;
    Ok(pool)
}
