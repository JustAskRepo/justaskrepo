// main.rs — the composition root, and nothing else.
//
// Five steps: config -> context -> event subscriptions -> serve -> drain. No
// routes, no handlers, no business logic. Adding a module touches exactly two
// lines here: one subscription, and (indirectly) one route file under
// infrastructure/http.

use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use anyhow::Result;
use justaskrepo::{
    infrastructure::{AppContext, config::AppConfig, http},
    modules,
};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::time::timeout;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// Total time the drain in step 5 will wait for every listener combined before
/// aborting what is left.
///
/// Deliberately short, because today it is spent in full every single time: the
/// reference cycle described in step 5 means no listener can ever observe its
/// channel closing, so the wait always ends in an abort. Anything longer is
/// latency added to every deploy in exchange for nothing.
///
/// It is not zero because the wait costs nothing to keep and becomes a real
/// graceful drain the moment that cycle is fixed — at which point raising it
/// again is the right move. Work in flight is unaffected either way:
/// `event_bus::dispatch` runs each handler in its own task, which outlives the
/// listener that spawned it.
const DRAIN_GRACE: Duration = Duration::from_secs(2);

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
    //    Flattened rather than chained so that adding a module is one line and
    //    an empty Vec from a module that listens to nothing costs nothing.
    let subscriptions: Vec<_> = [
        modules::auth::subscribe(&ctx).await,
        modules::installations::subscribe(&ctx).await,
        modules::webhooks::subscribe(&ctx).await,
    ]
    .into_iter()
    .flatten()
    .collect();

    // 4. Serve.
    let listener = TcpListener::bind(bind_addr).await?;
    info!(%bind_addr, %public_url, "server listening");

    axum::serve(
        listener,
        http::router(ctx).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    // 5. Drain — on a deadline, because an unbounded wait here can hang forever.
    //
    //    The obvious reasoning is that dropping the router drops the last
    //    AppContext, which closes the bus channels, which ends every listener.
    //    That is wrong as soon as a handler needs to publish: the listener's
    //    closure holds an AppContext, an AppContext holds an EventBus, and the
    //    bus owns the broadcast Sender — so the listener is waiting on a channel
    //    it is itself keeping open. `auth`'s uninstall handler is the first real
    //    subscriber and it closes exactly that cycle.
    //
    //    So the drain gets a budget and then aborts. Aborting a *listener* does
    //    not cancel work in flight: `event_bus::dispatch` runs each handler in
    //    its own task, and that task survives its listener. The budget is one
    //    total, not one per listener, so shutdown cannot scale with subscriber
    //    count.
    //
    //    The root fix is to stop deciding listener lifetime by reference count —
    //    a cancellation token, or publishing through a weak handle. Worth doing
    //    when the outbox reopens the bus contract; not worth blocking shutdown on
    //    until then.
    info!(
        listeners = subscriptions.len(),
        "waiting for event listeners"
    );

    let deadline = Instant::now() + DRAIN_GRACE;
    for mut handle in subscriptions {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match timeout(remaining, &mut handle).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => warn!(%error, "an event listener did not shut down cleanly"),
            Err(_) => {
                handle.abort();
                warn!(
                    grace = ?DRAIN_GRACE,
                    "event listener did not stop within the drain budget; aborted"
                );
            }
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
