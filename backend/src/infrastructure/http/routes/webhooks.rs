use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
};

use crate::{
    infrastructure::AppContext,
    modules::webhooks::{
        ReceiveGitHubWebhookCommand, WebhookResponse, handle_receive_github_webhook,
    },
    shared_kernel::error::AppError,
};

/// **Public — mounted outside the session layer, deliberately.**
///
/// GitHub carries no cookie and no `Origin`, so `require_session` would reject
/// every delivery and the CSRF check it performs would be meaningless here
/// anyway (ARCHITECTURE.md §5.2). The HMAC over the body is the authentication,
/// and it is stronger than a session for this purpose: it proves the request
/// came from someone holding the shared secret.
///
/// No rate limit either. The limiter keys on client address, GitHub delivers
/// from a large and changing set of them, and a 429 would silently drop real
/// events. An unsigned flood costs one HMAC each and no I/O at all.
pub fn routes() -> Router<AppContext> {
    Router::new().route("/webhooks/github", post(github))
}

/// `Bytes`, never `Json<T>`: the signature covers the exact bytes GitHub sent,
/// so anything that parses and re-encodes before verification turns every
/// genuine delivery into a rejection.
async fn github(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    let outcome = handle_receive_github_webhook(
        ReceiveGitHubWebhookCommand {
            event: header(&headers, "X-GitHub-Event"),
            delivery_id: header(&headers, "X-GitHub-Delivery"),
            signature: headers
                .get("X-Hub-Signature-256")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
            body,
        },
        &ctx,
    )
    .await?;

    // Both outcomes are 2xx. GitHub retries non-2xx on a schedule, so an event
    // we chose not to act on must still be accepted or it comes back forever.
    // Anything that *is* worth retrying leaves through `AppError` instead.
    Ok(match outcome {
        WebhookResponse::Handled => StatusCode::NO_CONTENT,
        WebhookResponse::Ignored => StatusCode::ACCEPTED,
    })
}

fn header(headers: &HeaderMap, name: &str) -> String {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}
