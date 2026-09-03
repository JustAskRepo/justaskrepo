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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use chrono::{DateTime, TimeDelta};

    use crate::modules::auth::domain::session::Session;
    use crate::modules::auth::infrastructure::test_support::{
        pool, session, unique_session, unique_user,
    };
    use crate::shared_kernel::types::SessionId;

    const IDLE: Duration = Duration::from_secs(60);
    const ABSOLUTE: Duration = Duration::from_secs(600);
    const THRESHOLD: Duration = Duration::from_secs(300);

    fn auth() -> Arc<AuthContext> {
        Arc::new(AuthContext {
            cookie_name: "__Host-session".to_owned(),
            absolute_ttl: ABSOLUTE,
            idle_ttl: IDLE,
            refresh_threshold: THRESHOLD,
            client_id: "id".to_owned(),
            client_secret: "secret".into(),
            redirect_uri: "http://localhost:8080/api/auth/github/callback".to_owned(),
        })
    }

    async fn store(pool: &deadpool_redis::Pool, session_id: &SessionId, session: &Session) {
        if let Err(error) =
            session_store::create_session(session_id, session, IDLE, ABSOLUTE, pool.clone()).await
        {
            panic!("create_session: {error}");
        }
    }

    async fn last_seen_at(pool: &deadpool_redis::Pool, session_id: &SessionId) -> DateTime<Utc> {
        match session_store::load_session(session_id, pool.clone()).await {
            Ok(Some(session)) => session.last_seen_at,
            other => panic!("expected a stored session, got {other:?}"),
        }
    }

    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn a_session_that_was_never_stored_is_unauthorized() {
        let pool = pool().await;

        let result = run(
            GetSessionQuery {
                session_id: unique_session(),
            },
            auth(),
            pool,
        )
        .await;

        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    /// Valkey's TTL is the *idle* timeout and gets pushed forward on activity,
    /// so it structurally cannot enforce the absolute cap — this check is the
    /// only thing that does. The record here is alive in Valkey for another
    /// minute and already past its cap, which is exactly the state a long-lived
    /// active session reaches on day 30.
    ///
    /// Rejecting it is half the job; deleting it is the other half, or the key
    /// lingers until its idle TTL and every request re-does this work.
    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn a_session_past_its_absolute_cap_is_rejected_and_deleted() {
        let pool = pool().await;
        let (user_id, session_id) = (unique_user(), unique_session());
        let expired = session(user_id, Duration::ZERO);

        store(&pool, &session_id, &expired).await;

        let result = run(
            GetSessionQuery {
                session_id: session_id.clone(),
            },
            auth(),
            pool.clone(),
        )
        .await;

        assert!(matches!(result, Err(AppError::Unauthorized)));
        match session_store::load_session(&session_id, pool).await {
            Ok(None) => {}
            other => panic!("the expired session outlived its rejection: {other:?}"),
        }
    }

    /// The throttle, from the side that must not write: a request arriving
    /// seconds after the last one costs a read and nothing else. Without this,
    /// every authenticated request is a Valkey write.
    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn a_recently_seen_session_is_not_rewritten() {
        let pool = pool().await;
        let (user_id, session_id) = (unique_user(), unique_session());
        let fresh = session(user_id, ABSOLUTE);

        store(&pool, &session_id, &fresh).await;
        if let Err(error) = run(
            GetSessionQuery {
                session_id: session_id.clone(),
            },
            auth(),
            pool.clone(),
        )
        .await
        {
            panic!("a fresh session should validate: {error}");
        }

        assert_eq!(
            last_seen_at(&pool, &session_id).await,
            fresh.last_seen_at,
            "the session was rewritten inside the refresh threshold"
        );
    }

    /// And from the side that must: once the threshold has passed, the session
    /// is written back — which is what re-arms the sliding idle TTL. Skip it and
    /// an active session expires as if it had been idle.
    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn a_session_idle_past_the_threshold_is_touched() {
        let pool = pool().await;
        let (user_id, session_id) = (unique_user(), unique_session());
        let mut stale = session(user_id, ABSOLUTE);
        stale.touch(Utc::now() - TimeDelta::minutes(10));

        store(&pool, &session_id, &stale).await;
        if let Err(error) = run(
            GetSessionQuery {
                session_id: session_id.clone(),
            },
            auth(),
            pool.clone(),
        )
        .await
        {
            panic!("a stale but unexpired session should validate: {error}");
        }

        assert!(
            last_seen_at(&pool, &session_id).await > stale.last_seen_at,
            "the session was not touched past the refresh threshold"
        );
    }
}
