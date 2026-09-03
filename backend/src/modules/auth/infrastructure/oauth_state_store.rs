use crate::shared_kernel::error::AppError;
use deadpool_redis::redis::cmd;

pub(in crate::modules::auth) async fn store_state_valkey(
    state: &str,
    valkey: deadpool_redis::Pool,
) -> Result<(), AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let ttl = std::time::Duration::from_secs(10 * 60); // 10 minutes
    cmd("SET")
        .arg(format!("oauth_state:{state}"))
        .arg(1)
        .arg("EX")
        .arg(ttl.as_secs())
        .query_async::<()>(&mut conn)
        .await
        .map_err(AppError::internal)?;
    Ok(())
}

pub(in crate::modules::auth) async fn verify_and_consume_state_valkey(
    state: &str,
    valkey: deadpool_redis::Pool,
) -> Result<(), AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let result: Option<String> = cmd("GETDEL")
        .arg(format!("oauth_state:{state}"))
        .query_async(&mut conn)
        .await
        .map_err(AppError::internal)?;

    if result.is_none() {
        tracing::warn!("oauth callback rejected: state missing or expired");
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::auth::infrastructure::test_support::{pool, unique_token};

    fn is_unauthorized(result: Result<(), AppError>) -> bool {
        matches!(result, Err(AppError::Unauthorized))
    }

    /// Single use is the whole point of the `state` key: it is the login-CSRF
    /// defence, and a `state` that verifies twice is a replayable callback.
    /// `GETDEL` is what makes the check and the consumption one operation —
    /// a `GET` followed by a `DEL` would let two concurrent callbacks both pass.
    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn a_state_verifies_once_and_the_replay_is_rejected() {
        let pool = pool().await;
        let state = unique_token();

        if let Err(error) = store_state_valkey(&state, pool.clone()).await {
            panic!("store_state_valkey: {error}");
        }

        assert!(
            verify_and_consume_state_valkey(&state, pool.clone())
                .await
                .is_ok()
        );
        assert!(
            is_unauthorized(verify_and_consume_state_valkey(&state, pool).await),
            "the same state verified twice — the callback is replayable"
        );
    }

    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn a_state_we_never_issued_is_rejected() {
        let pool = pool().await;

        assert!(is_unauthorized(
            verify_and_consume_state_valkey(&unique_token(), pool).await
        ));
    }
}
