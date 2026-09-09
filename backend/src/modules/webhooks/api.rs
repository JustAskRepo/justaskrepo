// webhooks/api.rs
//
// This is the ONLY public surface of this module.
//
// The module is a dispatcher, not a domain module: it owns no data, publishes
// no events, and answers no queries. Its single command turns a verified HTTP
// request into a call on whichever module owns the thing that changed.

use axum::body::Bytes;
use tokio::task::JoinHandle;

use crate::modules::webhooks::application::commands::receive_github_webhook;
use crate::{infrastructure::AppContext, shared_kernel::error::AppError};

// ─── Subscriptions ───────────────────────────────────────────────────────────

/// Empty, and structurally so: this module sits upstream of the bus rather than
/// on it. Present only so `main.rs` registers every module the same way.
pub async fn subscribe(_ctx: &AppContext) -> Vec<JoinHandle<()>> {
    Vec::new()
}

// ─── Commands ────────────────────────────────────────────────────────────────

// receive_github_webhook──────────────────────────────────────────────────────
/// One delivery from GitHub, unverified.
///
/// `body` is `Bytes` and must be the exact bytes received. The signature covers
/// them literally, so anything that parses and re-encodes on the way here —
/// axum's `Json` extractor included — changes the digest and turns every genuine
/// delivery into a rejection.
///
/// Holding an axum type is the one concession this module makes to the
/// framework, and it is `Bytes` rather than `Request` deliberately: the route
/// still does the extracting, and nothing here knows about headers, methods or
/// status codes.
#[derive(Debug)]
pub struct ReceiveGitHubWebhookCommand {
    pub event: String,
    pub delivery_id: String,
    pub signature: Option<String>,
    pub body: Bytes,
}

/// Whether anything was done. Both are successes — an event we do not act on
/// still has to be accepted, or GitHub retries it on a schedule forever.
#[derive(Debug, PartialEq, Eq)]
pub enum WebhookResponse {
    Handled,
    Ignored,
}

#[tracing::instrument(skip_all)]
pub async fn handle_receive_github_webhook(
    cmd: ReceiveGitHubWebhookCommand,
    ctx: &AppContext,
) -> Result<WebhookResponse, AppError> {
    receive_github_webhook::run(cmd, ctx.webhooks.clone(), ctx).await
}
