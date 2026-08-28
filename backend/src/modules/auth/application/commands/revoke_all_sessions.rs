use crate::{modules::auth::api::RevokeAllSessionsCommand, shared_kernel::error::AppError};

use crate::modules::auth::infrastructure::session_store;

/// Deliberately not idempotency-checked: revoking zero sessions is a success,
/// not a `NotFound`. The caller asked for "none of my sessions remain", and
/// that is true either way.
#[tracing::instrument(skip_all, err)]
pub(crate) async fn run(
    cmd: RevokeAllSessionsCommand,
    valkey: deadpool_redis::Pool,
) -> Result<(), AppError> {
    let revoked = session_store::delete_all_sessions(cmd.user_id, valkey).await?;

    tracing::info!(user_id = cmd.user_id.0, revoked, "all sessions revoked");
    Ok(())
}
