use chrono::Utc;

use crate::{
    infrastructure::event_bus::EventBus, modules::auth::api::RevokeSessionCommand,
    shared_kernel::error::AppError,
};

use crate::modules::auth::domain::{events::UserSessionsRevokedEvent, session::RevocationScope};
use crate::modules::auth::infrastructure::session_store;

/// Deleting the record *is* the revocation. There is no "logged out" flag to
/// set and nothing left to expire — the credential stops working the instant
/// the key is gone. That is the property session-in-Valkey buys over a JWT,
/// which stays valid until its own expiry no matter what the server thinks.
#[tracing::instrument(skip_all, err)]
pub(crate) async fn run(
    cmd: RevokeSessionCommand,
    valkey: deadpool_redis::Pool,
    events: EventBus,
) -> Result<(), AppError> {
    session_store::delete_session(&cmd.session_id, cmd.user_id, valkey).await?;

    tracing::info!(user_id = cmd.user_id.0, "session revoked");

    events
        .publish(UserSessionsRevokedEvent::new(
            cmd.user_id,
            RevocationScope::One,
            Utc::now(),
        ))
        .await;

    Ok(())
}
