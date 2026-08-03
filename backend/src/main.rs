// main.rs — the composition root, and nothing else.
//
// Four steps: config -> context -> event subscriptions -> serve. No routes, no
// handlers, no business logic. Adding a module touches exactly two lines here:
// one subscription, and (indirectly) one route file under infrastructure/http.

use anyhow::Result;
use justaskrepo::infrastructure::{AppContext, config::AppConfig, http};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("justaskrepo=debug,tower_http=debug,info")),
        )
        .init();

    // 1. Config. Every environment variable in the process is read here and
    //    nowhere else; a bad one kills startup rather than the first request.
    let config = AppConfig::from_env()?;
    let bind_addr = config.server.bind_addr;
    let public_url = &config.server.public_url;

    // 2. Context — pools and shared handles, built once and cloned everywhere.
    let ctx = AppContext::new(&config).await?;

    sqlx::migrate!().run(&ctx.db).await?;
    info!("database migrations applied");

    // 3. Event subscriptions.
    //    TODO: modules::auth::subscribe(&ctx) etc. once the event bus exists.
    //    Prefer returning JoinHandles so shutdown can await them — cheap now,
    //    annoying to retrofit at five modules.

    // 4. Serve.
    let listener = TcpListener::bind(bind_addr).await?;
    info!(%bind_addr, %public_url, "server listening");
    axum::serve(listener, http::router(ctx)).await?;

    Ok(())
}
