use std::sync::Arc;

use chrono::Utc;

use crate::{
    infrastructure::context::AuthContext,
    modules::auth::api::{GetSessionQuery, SessionResponse},
    shared_kernel::error::AppError,
};

use crate::modules::auth::infrastructure::session_store;

#[tracing::instrument(skip_all, err)]
pub(crate) async fn run(
    query: GetSessionQuery,
    auth: Arc<AuthContext>,
    valkey: deadpool_redis::Pool,
) -> Result<SessionResponse, AppError> {
    let Some(mut session) = session_store::load_session(&query.session_id, valkey.clone()).await?
    else {
        return Err(AppError::Unauthorized);
    };

    let now = Utc::now();
    if session.is_expired(now) {
        session_store::delete_session(&query.session_id, session.user_id, valkey).await?;
        return Err(AppError::Unauthorized);
    }

    if session.needs_refresh(now, auth.refresh_threshold) {
        session.touch(now);
        session_store::touch_session(&query.session_id, &session, auth.idle_ttl, valkey).await?;
    }

    Ok(SessionResponse {
        user_id: session.user_id,
        github_id: session.github_id,
    })
}
