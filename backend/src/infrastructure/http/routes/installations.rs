use axum::{
    Router,
    extract::{Query, State},
    response::Redirect,
    routing::get,
};
use serde::Deserialize;

use crate::{
    infrastructure::AppContext,
    modules::installations::{GetInstallationUrlQuery, handle_get_installation_url},
    shared_kernel::error::AppError,
};

/// Mounted behind `require_session` in `http::router`.
pub fn protected_routes() -> Router<AppContext> {
    Router::new().route("/installations/new", get(new_installation))
}

/// Public — mounted outside the session layer.
///
/// This is the App's **Setup URL**, where GitHub sends a user after they finish
/// installing. It must work without a session: someone can install the App from
/// its GitHub page having never signed in here, and answering them with a 401
/// strands them at the end of a flow they just completed.
pub fn routes() -> Router<AppContext> {
    Router::new().route("/installations/setup", get(setup_complete))
}

/// The frontend navigates the whole page here rather than fetching it, so the
/// browser follows this redirect to GitHub's installation screen. Keeping the
/// GitHub URL server-side is the same shape as sign-in: the frontend holds no
/// GitHub URLs of its own.
async fn new_installation(State(ctx): State<AppContext>) -> Result<Redirect, AppError> {
    let res = handle_get_installation_url(GetInstallationUrlQuery, &ctx).await?;
    Ok(Redirect::temporary(&res.install_url))
}

/// Where GitHub returns a user after they install the App.
///
/// **It grants nothing and writes nothing.** GitHub appends `installation_id`
/// here and its own documentation warns the value is attacker-supplied: hand a
/// victim this URL carrying your installation id and a naive handler would
/// attach their session to your grant. The documented defence is to check the id
/// against `GET /user/installations`, which needs a user access token this
/// request does not have — so rather than perform a check we cannot actually
/// perform, the parameters are ignored entirely (ADR-009 decision 3).
///
/// What it does instead is send the user back through sign-in. For an App they
/// have already authorized that is a silent redirect, and it is the one thing
/// that produces a fresh user token — which is what lets reconciliation run so
/// the new grant appears immediately. Without this hop, installing the App
/// leaves the dashboard empty until the user next happens to sign in, because
/// the webhook path writes `installation_repositories` while the dashboard reads
/// `user_repositories` (migration 0003).
///
/// The login callback then picks the destination as it always does.
async fn setup_complete(Query(query): Query<SetupQuery>) -> Redirect {
    // Debug only, and never acted on: an untrusted value recorded for
    // diagnosis, not read for meaning.
    tracing::debug!(
        installation_id = ?query.installation_id,
        setup_action = ?query.setup_action,
        "returned from the GitHub App setup flow"
    );

    Redirect::temporary("/api/auth/github")
}

/// Captured only so the values can be logged and discarded. Both optional:
/// nothing here depends on GitHub sending either.
#[derive(Debug, Deserialize)]
struct SetupQuery {
    installation_id: Option<String>,
    setup_action: Option<String>,
}
