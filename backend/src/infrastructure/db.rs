use secrecy::ExposeSecret;
use sqlx::PgPool;

pub async fn connect_db(db_config: &super::config::DatabaseConfig) -> anyhow::Result<PgPool> {
    let pool = PgPool::connect(db_config.url.expose_secret()).await?;
    Ok(pool)
}
