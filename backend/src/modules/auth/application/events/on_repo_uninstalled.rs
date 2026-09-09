use crate::{
    infrastructure::AppContext, modules::auth::api::RevokeAllSessionsCommand,
    modules::installations::RepoUninstalledEvent, shared_kernel::error::AppError,
};

use crate::modules::auth::application::commands::revoke_all_sessions;

/// The App was uninstalled, so this user's sessions stop working.
///
/// This closes the gap `AUTHENTICATION.md` has carried since the session design
/// was written: revoking access at GitHub removed the App's reach into the
/// repositories, but left the user signed in here. The trigger arrives as a
/// webhook, `installations` turns it into an event, and this is where it lands.
///
/// It reuses the command `POST /api/auth/logout/all` already builds rather than
/// growing revocation logic of its own — there is one way to end a user's
/// sessions, and this is another caller of it.
///
/// Handling the same event twice is harmless by construction: revoking zero
/// sessions is a success, not a `NotFound`. That matters more than it looks,
/// because the outbox upgrade in ADR-004 turns delivery from at-most-once into
/// at-least-once, and this handler is already ready for it.
#[tracing::instrument(skip_all, fields(user_id = event.user_id.0), err)]
pub(crate) async fn run(event: RepoUninstalledEvent, ctx: AppContext) -> Result<(), AppError> {
    revoke_all_sessions::run(
        RevokeAllSessionsCommand {
            user_id: event.user_id,
        },
        ctx.valkey.clone(),
        ctx.events.clone(),
    )
    .await
}
