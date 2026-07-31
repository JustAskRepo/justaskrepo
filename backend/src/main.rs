use anyhow::Result;
use axum::{Router, routing::get};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = Router::new().route("/", get(hello_world));

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    info!(addr = "0.0.0.0:8080", "server listening");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn hello_world() -> &'static str {
    "Hello, World!"
}
