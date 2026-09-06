// main.rs — the composition root, and nothing else.
//
// Five steps: config -> context -> event subscriptions -> serve -> drain. No
// routes, no handlers, no business logic. Adding a module touches exactly two
// lines here: one subscription, and (indirectly) one route file under
// infrastructure/http.

use std::net::SocketAddr;

use anyhow::Result;
use justaskrepo::{
    infrastructure::{AppContext, config::AppConfig, http},
    modules,
};
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{error, info, warn};
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

    // 3. Event subscriptions — each module hands back its listener handles.
    let subscriptions = modules::auth::subscribe(&ctx).await;

    // 4. Serve.
    let listener = TcpListener::bind(bind_addr).await?;
    info!(%bind_addr, %public_url, "server listening");

    axum::serve(
        listener,
        http::router(ctx).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    // 5. Drain. The router is gone, so the channels are closed and listeners end.
    info!(
        listeners = subscriptions.len(),
        "waiting for event listeners"
    );
    for handle in subscriptions {
        if let Err(error) = handle.await {
            warn!(%error, "an event listener did not shut down cleanly");
        }
    }

    info!("shutdown complete");
    Ok(())
}

/// Resolves when the process is asked to stop. SIGTERM is the one that matters
/// in a container; Ctrl+C is the one that matters on a laptop.
///
/// A signal handler that cannot be installed is logged and then ignored rather
/// than treated as a shutdown request — completing here would stop the server
/// the instant it started.
async fn shutdown_signal() {
    let interrupt = async {
        match signal::ctrl_c().await {
            Ok(()) => info!("interrupt received"),
            Err(error) => {
                error!(%error, "cannot listen for Ctrl+C; ignoring it");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
                info!("termination signal received");
            }
            Err(error) => {
                error!(%error, "cannot listen for SIGTERM; ignoring it");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = interrupt => {}
        () = terminate => {}
    }
}
