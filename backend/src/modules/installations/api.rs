// installations/api.rs
//
// This is the ONLY public surface of this module.
// All Commands, Queries, and their handler functions live here.
// Handlers are thin — they delegate to application/ use cases.
//
// The module owns one question: which repositories may this user see, and by
// what authority may we read them. ADR-009 records where each half of the
// answer comes from.

use chrono::{DateTime, Utc};
use secrecy::SecretString;
use tokio::task::JoinHandle;

use crate::modules::installations::application::commands::{
    link_user_installations, refresh_user_installations, remove_installation, sync_installation,
};
use crate::modules::installations::application::queries::{
    get_installation_status, get_installation_token, get_installation_url, list_user_repositories,
};
use crate::{
    infrastructure::AppContext,
    shared_kernel::{
        error::AppError,
        types::{InstallationId, RepoFullName, RepositoryId, UserId},
    },
};

// ─── Events ──────────────────────────────────────────────────────────────────

pub use crate::modules::installations::domain::events::{RepoInstalledEvent, RepoUninstalledEvent};

// ─── Subscriptions ───────────────────────────────────────────────────────────

/// Empty by decision rather than by omission — the distinction matters here.
///
/// This module is *called*, not subscribed. Authorization is reconciled from a
/// user access token at login, and the bus carries IDs and never credentials,
/// so a subscriber to `UserAuthenticatedEvent` would arrive holding a user id
/// and nothing to ask GitHub with. The composition root calls the command
/// instead (ADR-009 decision 2, ARCHITECTURE.md §5.1).
///
/// It exists anyway so `main.rs` registers every module identically, and returns
/// a `Vec` for the reason `auth::subscribe` does: widening the signature later
/// would touch every module at once.
pub async fn subscribe(_ctx: &AppContext) -> Vec<JoinHandle<()>> {
    Vec::new()
}

// ─── Commands ────────────────────────────────────────────────────────────────

// handle_link_user_installations─────────────────────────────────────────────
/// Reconciles what GitHub says this user may see against what we have stored.
///
/// Carries a live user access token because it is the only credential that can
/// answer the question, and the login callback is the only moment one exists —
/// `auth` spends it on the profile fetch and drops it (ADR-008 decision 3). It
/// is `SecretString` rather than `String` so that formatting this struct, which
/// derives `Debug` like every Command, cannot put a credential in the logs.
///
/// Called by the composition root, never by a subscriber: an event carries IDs
/// and no credentials, so a listener would arrive unable to do the work
/// (ADR-009 decision 2).
#[derive(Debug)]
pub struct LinkUserInstallationsCommand {
    pub user_id: UserId,
    pub user_token: SecretString,
}

#[derive(Debug)]
pub struct LinkUserInstallationsResponse {
    pub installations: usize,
    pub repositories: usize,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_link_user_installations(
    cmd: LinkUserInstallationsCommand,
    ctx: &AppContext,
) -> Result<LinkUserInstallationsResponse, AppError> {
    link_user_installations::run(
        cmd,
        ctx.installations.clone(),
        ctx.valkey.clone(),
        ctx.http.clone(),
        ctx.db.clone(),
        ctx.events.clone(),
    )
    .await
}

// handle_sync_installation───────────────────────────────────────────────────
/// Re-read one installation from GitHub and replace what we hold for it.
///
/// Takes an id and nothing else. The webhook that triggers this carries a full
/// payload describing the change, and this command deliberately ignores it:
/// deliveries are at-least-once and unordered, so the payload is a claim about
/// a moment that may already have passed. One extra request buys immunity to
/// every ordering bug that trusting it would invite.
///
/// It therefore covers `installation.created`, `.suspend`, `.unsuspend`,
/// `.new_permissions_accepted` and both `installation_repositories` actions —
/// after a re-fetch they all mean the same thing.
#[derive(Debug)]
pub struct SyncInstallationCommand {
    pub installation_id: InstallationId,
}

#[derive(Debug)]
pub struct SyncInstallationResponse {
    pub repositories: usize,
    /// Repositories we had not seen before, and so the number of
    /// `RepoInstalledEvent`s published.
    pub newly_installed: usize,
    pub suspended: bool,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_sync_installation(
    cmd: SyncInstallationCommand,
    ctx: &AppContext,
) -> Result<SyncInstallationResponse, AppError> {
    sync_installation::run(
        cmd,
        ctx.installations.clone(),
        ctx.valkey.clone(),
        ctx.http.clone(),
        ctx.db.clone(),
        ctx.events.clone(),
    )
    .await
}

// handle_refresh_user_installations──────────────────────────────────────────
/// Brings this user's grants up to date, then returns how much work it did.
///
/// Called by the read path — `GET /api/repositories` — because webhook delivery
/// is not a freshness guarantee, and a dashboard listing a repository someone
/// has removed is an over-grant rather than a cosmetic lag. Throttled per
/// installation, so a polling client does not become a polling client of GitHub.
///
/// A Command despite being triggered by a read: it writes, and ADR-002 draws the
/// line there. The route calls it and then the Query, which stays pure — the
/// composition root doing orchestration a module may not (ARCHITECTURE.md §5.1).
#[derive(Debug)]
pub struct RefreshUserInstallationsCommand {
    pub user_id: UserId,
    /// Ignore the throttle. Set only for an explicit user action — someone who
    /// pressed Refresh has already seen the stale answer, so making them wait
    /// out a window meant for background polling is backwards.
    pub force: bool,
}

#[derive(Debug)]
pub struct RefreshUserInstallationsResponse {
    pub synced: usize,
    /// Installations inside their throttle window. Not a failure — the stored
    /// answer was refreshed recently enough.
    pub skipped: usize,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_refresh_user_installations(
    cmd: RefreshUserInstallationsCommand,
    ctx: &AppContext,
) -> Result<RefreshUserInstallationsResponse, AppError> {
    refresh_user_installations::run(
        cmd,
        ctx.installations.clone(),
        ctx.valkey.clone(),
        ctx.http.clone(),
        ctx.db.clone(),
        ctx.events.clone(),
    )
    .await
}

// handle_remove_installation─────────────────────────────────────────────────
/// The App was uninstalled. Drops the grant and announces every user who held
/// access through it.
///
/// Removing something already gone is success. GitHub redelivers, and an
/// idempotent delete is what keeps a redelivery from becoming an error.
#[derive(Debug)]
pub struct RemoveInstallationCommand {
    pub installation_id: InstallationId,
}

#[derive(Debug)]
pub struct RemoveInstallationResponse {
    /// How many `RepoUninstalledEvent`s were published — one per user who held
    /// access. Zero is normal: someone can install the App without ever
    /// signing in.
    pub affected_users: usize,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_remove_installation(
    cmd: RemoveInstallationCommand,
    ctx: &AppContext,
) -> Result<RemoveInstallationResponse, AppError> {
    remove_installation::run(cmd, ctx.valkey.clone(), ctx.db.clone(), ctx.events.clone()).await
}

// ─── Queries ─────────────────────────────────────────────────────────────────

// handle_get_installation_status──────────────────────────────────────────────
/// Whether this user has any installation that currently grants anything — the
/// branch the post-login redirect has been missing.
#[derive(Debug)]
pub struct GetInstallationStatusQuery {
    pub user_id: UserId,
}

#[derive(Debug)]
pub struct InstallationStatusResponse {
    pub has_any: bool,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_get_installation_status(
    query: GetInstallationStatusQuery,
    ctx: &AppContext,
) -> Result<InstallationStatusResponse, AppError> {
    get_installation_status::run(query, ctx.db.clone()).await
}

// handle_list_user_repositories──────────────────────────────────────────────
/// Every repository this user may reach, with no index state attached.
///
/// The dashboard's row mixes this module's answer with `indexing`'s status, and
/// neither module may read the other's tables. The route handler assembles the
/// two — that is the composition root's privilege and exactly the escape hatch
/// ARCHITECTURE.md §5.1 exists for.
#[derive(Debug)]
pub struct ListUserRepositoriesQuery {
    pub user_id: UserId,
}

/// `installation_id` travels with each row because it is the other half of the
/// module's question: it names the grant whose token `indexing` must mint to
/// read this repository.
#[derive(Debug)]
pub struct UserRepositoryResponse {
    pub repository_id: RepositoryId,
    pub installation_id: InstallationId,
    pub owner_login: String,
    pub name: String,
    pub full_name: RepoFullName,
    pub is_private: bool,
    pub default_branch: String,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_list_user_repositories(
    query: ListUserRepositoriesQuery,
    ctx: &AppContext,
) -> Result<Vec<UserRepositoryResponse>, AppError> {
    list_user_repositories::run(query, ctx.db.clone()).await
}

// handle_get_installation_token──────────────────────────────────────────────
/// A short-lived credential for reading one installation's repositories.
///
/// **Not reachable over HTTP.** There is no route for this and there must not
/// be — it exists so `indexing` can do repository I/O without reaching into this
/// module's `infrastructure/`, which architecture rule 1 forbids. Designing the
/// door now is cheaper than retrofitting it with `indexing` half-written.
///
/// **It performs no authorization**, and that is deliberate rather than
/// overlooked. The caller has already established that the work is permitted —
/// a user could only ask to index a repository `ListUserRepositoriesQuery`
/// showed them, and a webhook-triggered job has no user to check at all. Adding
/// a `user_id` here would make the second case impossible to express.
///
/// It is a Query by ADR-002's test — it publishes no domain event and changes
/// no state we own — though it does cost a call to GitHub on a cache miss.
#[derive(Debug)]
pub struct GetInstallationTokenQuery {
    pub installation_id: InstallationId,
}

/// `expires_at` travels with the token so a long job can decide to re-ask rather
/// than discover the expiry mid-clone.
#[derive(Debug)]
pub struct InstallationTokenResponse {
    pub token: SecretString,
    pub expires_at: DateTime<Utc>,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_get_installation_token(
    query: GetInstallationTokenQuery,
    ctx: &AppContext,
) -> Result<InstallationTokenResponse, AppError> {
    get_installation_token::run(
        query,
        ctx.installations.clone(),
        ctx.valkey.clone(),
        ctx.http.clone(),
    )
    .await
}

// handle_get_installation_url─────────────────────────────────────────────────
/// Where to send a user who wants to install the App.
///
/// A Query, not a Command, because nothing is written — which is what separates
/// it from `auth`'s `StartGithubLoginCommand`, whose whole job is storing a
/// single-use `state` before handing back a URL. There is no equivalent secret
/// here: GitHub identifies the installation to us afterwards, over a channel the
/// user cannot forge (ADR-009 decision 3 explains why the redirect it lands on
/// is trusted with nothing).
#[derive(Debug)]
pub struct GetInstallationUrlQuery;

#[derive(Debug)]
pub struct InstallationUrlResponse {
    pub install_url: String,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_get_installation_url(
    query: GetInstallationUrlQuery,
    ctx: &AppContext,
) -> Result<InstallationUrlResponse, AppError> {
    get_installation_url::run(query, ctx.installations.clone())
}
