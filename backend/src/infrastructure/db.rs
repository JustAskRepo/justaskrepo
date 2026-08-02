use secrecy::ExposeSecret;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub async fn connect_db(db_config: &super::config::DatabaseConfig) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(db_config.max_connections)
        .acquire_timeout(db_config.acquire_timeout)
        .connect(db_config.url.expose_secret())
        .await?;
    Ok(pool)
}
