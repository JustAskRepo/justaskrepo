// <module_name>/api.rs
//
// This is the ONLY public surface of this module.
// All Commands, Queries, and their handler functions live here.
// Handlers are thin — they delegate to application/ use cases.

use secrecy::SecretString;
use tokio::task::JoinHandle;

use crate::modules::auth::application::commands::{
    complete_github_login, revoke_all_sessions, revoke_session, start_github_login,
};
use crate::modules::auth::application::events::on_repo_uninstalled;
use crate::modules::auth::application::queries::{get_session, get_user_profile};
use crate::modules::installations::RepoUninstalledEvent;
use crate::{
    infrastructure::AppContext,
    shared_kernel::{
        error::AppError,
        types::{GitHubId, SessionId, UserId},
    },
};

// ─── Events ──────────────────────────────────────────────────────────────────

pub use crate::modules::auth::domain::events::{UserAuthenticatedEvent, UserSessionsRevokedEvent};
pub use crate::modules::auth::domain::session::RevocationScope;

// ─── Subscriptions ───────────────────────────────────────────────────────────

/// Starts this module's listeners and hands their task handles to `main.rs`,
/// which waits for them on shutdown rather than cutting them off mid-handler.
///
/// One listener: `RepoUninstalledEvent` from `installations`, which revokes
/// every session belonging to a user whose App installation is gone.
///
/// Naming another module's event type is the one dependency the bus does not
/// remove, and it is deliberate. An event is part of a module's published
/// contract (ARCHITECTURE.md §6) — `installations` re-exports this one from its
/// own `api.rs`, so this is a type dependency on a public surface, not a reach
/// into another module's internals. What it is *not* is a call: `installations`
/// has no idea anyone is listening, and deleting `auth` tomorrow would not
/// change a line of it.
///
/// A `Vec` because a module ends up listening to more than one event, and
/// widening this signature later would touch `main.rs` and every module that
/// had returned something narrower.
pub async fn subscribe(ctx: &AppContext) -> Vec<JoinHandle<()>> {
    let events = ctx.events.clone();
    let ctx = ctx.clone();

    vec![
        events
            .subscribe(move |event: RepoUninstalledEvent| {
                on_repo_uninstalled::run(event, ctx.clone())
            })
            .await,
    ]
}

// ─── Commands ────────────────────────────────────────────────────────────────

// handle_start_github_login─────────────────────────────────────────────────────────────────
#[derive(Debug)]
pub struct StartGithubLoginCommand;

#[derive(Debug)]
pub struct StartGithubLoginResponse {
    pub authorize_url: String,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_start_github_login(
    cmd: StartGithubLoginCommand,
    ctx: &AppContext,
) -> Result<StartGithubLoginResponse, AppError> {
    let authorize_url = start_github_login::run(cmd, ctx.auth.clone(), ctx.valkey.clone()).await?;
    Ok(StartGithubLoginResponse { authorize_url })
}

// handle_complete_github_login─────────────────────────────────────────────────────────────────
#[derive(Debug)]
pub struct CompleteGithubLoginCommand {
    pub state: SecretString,
    pub code: SecretString,
    pub ip: String,
    pub user_agent: String,
}

/// Carries the user access token out of the module, and nothing stores it.
///
/// ADR-008 decision 3 spends this token on the profile fetch and drops it. It
/// now has one more job before it is dropped — telling `installations` which
/// grants this human may see — and the composition root is where that handoff
/// happens, because a module may not call another module (ARCHITECTURE.md
/// §5.1) and an event may not carry a credential (ADR-009 decision 2).
///
/// `SecretString`, not `String`: this struct derives `Debug`, and a `tracing`
/// call that formats it must print `[REDACTED]`. Widening that type is a
/// contract change, not a cleanup.
#[derive(Debug)]
pub struct CompleteGithubLoginResponse {
    pub session_id: SessionId,
    pub user_id: UserId,
    pub user_token: SecretString,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_complete_github_login(
    cmd: CompleteGithubLoginCommand,
    ctx: &AppContext,
) -> Result<CompleteGithubLoginResponse, AppError> {
    complete_github_login::run(
        cmd,
        ctx.auth.clone(),
        ctx.valkey.clone(),
        ctx.http.clone(),
        ctx.db.clone(),
        ctx.events.clone(),
    )
    .await
}
// handle_revoke_session───────────────────────────────────────────────────────
/// Carries the user id as well as the session id because revocation is two
/// writes: drop the record, and drop its entry from that user's session index.
/// Passing it in beats re-reading the session we are about to delete.
#[derive(Debug)]
pub struct RevokeSessionCommand {
    pub session_id: SessionId,
    pub user_id: UserId,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_revoke_session(
    cmd: RevokeSessionCommand,
    ctx: &AppContext,
) -> Result<(), AppError> {
    revoke_session::run(cmd, ctx.valkey.clone(), ctx.events.clone()).await
}

// handle_revoke_all_sessions──────────────────────────────────────────────────
#[derive(Debug)]
pub struct RevokeAllSessionsCommand {
    pub user_id: UserId,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_revoke_all_sessions(
    cmd: RevokeAllSessionsCommand,
    ctx: &AppContext,
) -> Result<(), AppError> {
    revoke_all_sessions::run(cmd, ctx.valkey.clone(), ctx.events.clone()).await
}

// ─── Queries ─────────────────────────────────────────────────────────────────

// handle_get_session──────────────────────────────────────────────────────────
#[derive(Debug)]
pub struct GetSessionQuery {
    pub session_id: SessionId,
}

#[derive(Debug)]
pub struct SessionResponse {
    pub user_id: UserId,
    pub github_id: GitHubId,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_get_session(
    query: GetSessionQuery,
    ctx: &AppContext,
) -> Result<SessionResponse, AppError> {
    get_session::run(query, ctx.auth.clone(), ctx.valkey.clone()).await
}

// handle_get_user_profile─────────────────────────────────────────────────────
#[derive(Debug)]
pub struct GetUserProfileQuery {
    pub user_id: UserId,
}

#[derive(Debug)]
pub struct UserProfileResponse {
    pub user_id: UserId,
    pub github_id: GitHubId,
    pub username: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_get_user_profile(
    query: GetUserProfileQuery,
    ctx: &AppContext,
) -> Result<UserProfileResponse, AppError> {
    get_user_profile::run(query, ctx.db.clone()).await
}
