use std::time::Duration;

use deadpool_redis::redis::{cmd, pipe};

use crate::modules::auth::domain::session::Session;
use crate::shared_kernel::{
    error::AppError,
    types::{SessionId, UserId},
};

fn session_key(session_id: &str) -> String {
    format!("session:{session_id}")
}

fn user_index_key(user_id: i64) -> String {
    format!("user_sessions:{user_id}")
}

#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn create_session(
    session_id: &SessionId,
    session: &Session,
    idle_ttl: Duration,
    absolute_ttl: Duration,
    valkey: deadpool_redis::Pool,
) -> Result<(), AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let record = serde_json::to_string(session).map_err(AppError::internal)?;
    let index_key = user_index_key(session.user_id.0);

    pipe()
        .atomic()
        .cmd("SET")
        .arg(session_key(&session_id.0))
        .arg(record)
        .arg("EX")
        .arg(idle_ttl.as_secs())
        .ignore()
        .cmd("SADD")
        .arg(&index_key)
        .arg(session_id.0.as_str())
        .ignore()
        .cmd("EXPIRE")
        .arg(&index_key)
        .arg(absolute_ttl.as_secs())
        .ignore()
        .query_async::<()>(&mut conn)
        .await
        .map_err(AppError::internal)?;

    Ok(())
}

#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn load_session(
    session_id: &SessionId,
    valkey: deadpool_redis::Pool,
) -> Result<Option<Session>, AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let record: Option<String> = cmd("GET")
        .arg(session_key(&session_id.0))
        .query_async(&mut conn)
        .await
        .map_err(AppError::internal)?;

    record
        .map(|record| serde_json::from_str(&record).map_err(AppError::internal))
        .transpose()
}

#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn touch_session(
    session_id: &SessionId,
    session: &Session,
    idle_ttl: Duration,
    valkey: deadpool_redis::Pool,
) -> Result<(), AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let record = serde_json::to_string(session).map_err(AppError::internal)?;

    cmd("SET")
        .arg(session_key(&session_id.0))
        .arg(record)
        .arg("EX")
        .arg(idle_ttl.as_secs())
        .query_async::<()>(&mut conn)
        .await
        .map_err(AppError::internal)?;

    Ok(())
}

#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn delete_session(
    session_id: &SessionId,
    user_id: UserId,
    valkey: deadpool_redis::Pool,
) -> Result<(), AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;

    pipe()
        .atomic()
        .cmd("DEL")
        .arg(session_key(&session_id.0))
        .ignore()
        .cmd("SREM")
        .arg(user_index_key(user_id.0))
        .arg(session_id.0.as_str())
        .ignore()
        .query_async::<()>(&mut conn)
        .await
        .map_err(AppError::internal)?;

    Ok(())
}

/// "Log out everywhere" — bounded by the per-user index rather than a scan of
/// the keyspace, which is the entire reason that index exists.
///
/// Reads the index, deletes what it names, then removes **exactly those ids**
/// from the index — `SREM` of what we read, not `DEL` of the whole key. A login
/// landing between the read and the write adds its id to the same set, and
/// dropping the key wholesale would unindex that brand-new session while
/// leaving its record alive: still usable, but invisible to the next bulk
/// revocation. Removing only what we saw leaves the newcomer indexed.
#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn delete_all_sessions(
    user_id: UserId,
    valkey: deadpool_redis::Pool,
) -> Result<usize, AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;
    let index_key = user_index_key(user_id.0);

    let session_ids: Vec<String> = cmd("SMEMBERS")
        .arg(&index_key)
        .query_async(&mut conn)
        .await
        .map_err(AppError::internal)?;

    // `SREM key` with no members is an error, not a no-op.
    if session_ids.is_empty() {
        return Ok(0);
    }

    let mut batch = pipe();
    let batch = batch.atomic();
    for session_id in &session_ids {
        batch.cmd("DEL").arg(session_key(session_id)).ignore();
    }

    batch
        .cmd("SREM")
        .arg(&index_key)
        .arg(&session_ids)
        .ignore()
        .query_async::<()>(&mut conn)
        .await
        .map_err(AppError::internal)?;

    Ok(session_ids.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::auth::infrastructure::test_support::{
        pool, session, unique_session, unique_user,
    };

    const IDLE_SECS: i64 = 60;
    const ABSOLUTE_SECS: i64 = 600;
    const IDLE: Duration = Duration::from_secs(IDLE_SECS as u64);
    const ABSOLUTE: Duration = Duration::from_secs(ABSOLUTE_SECS as u64);

    async fn store(pool: &deadpool_redis::Pool, session_id: &SessionId, user_id: UserId) {
        let session = session(user_id, ABSOLUTE);
        if let Err(error) = create_session(session_id, &session, IDLE, ABSOLUTE, pool.clone()).await
        {
            panic!("create_session: {error}");
        }
    }

    async fn query<T: deadpool_redis::redis::FromRedisValue + Default>(
        pool: &deadpool_redis::Pool,
        command: &mut deadpool_redis::redis::Cmd,
    ) -> T {
        let mut conn = match pool.get().await {
            Ok(conn) => conn,
            Err(error) => panic!("lost the Valkey connection: {error}"),
        };
        command.query_async(&mut conn).await.unwrap_or_default()
    }

    async fn ttl_of(pool: &deadpool_redis::Pool, key: &str) -> i64 {
        query(pool, cmd("TTL").arg(key)).await
    }

    async fn record_exists(pool: &deadpool_redis::Pool, session_id: &SessionId) -> bool {
        query::<i64>(pool, cmd("EXISTS").arg(session_key(&session_id.0))).await == 1
    }

    async fn is_indexed(
        pool: &deadpool_redis::Pool,
        user_id: UserId,
        session_id: &SessionId,
    ) -> bool {
        query::<i64>(
            pool,
            cmd("SISMEMBER")
                .arg(user_index_key(user_id.0))
                .arg(session_id.0.as_str()),
        )
        .await
            == 1
    }

    /// The two TTLs are not interchangeable: the record carries the sliding idle
    /// timeout, the index carries the absolute cap. Swapped, a session record
    /// outlives the idle timeout by weeks and the sliding expiry is gone.
    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn the_record_expires_on_the_idle_ttl_and_the_index_on_the_cap() {
        let pool = pool().await;
        let (user_id, session_id) = (unique_user(), unique_session());

        store(&pool, &session_id, user_id).await;

        let record_ttl = ttl_of(&pool, &session_key(&session_id.0)).await;
        let index_ttl = ttl_of(&pool, &user_index_key(user_id.0)).await;

        assert!(
            (1..=IDLE_SECS).contains(&record_ttl),
            "record TTL {record_ttl}s is not the {IDLE_SECS}s idle timeout"
        );
        assert!(
            (IDLE_SECS + 1..=ABSOLUTE_SECS).contains(&index_ttl),
            "index TTL {index_ttl}s is not the {ABSOLUTE_SECS}s cap — the two TTLs look swapped"
        );
    }

    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn a_stored_session_round_trips() {
        let pool = pool().await;
        let (user_id, session_id) = (unique_user(), unique_session());

        store(&pool, &session_id, user_id).await;

        match load_session(&session_id, pool.clone()).await {
            Ok(Some(session)) => assert_eq!(session.user_id, user_id),
            other => panic!("expected the session we just stored, got {other:?}"),
        }
        assert!(is_indexed(&pool, user_id, &session_id).await);
    }

    /// An unknown session id is an answer, not a failure — `require_session`
    /// turns the `None` into a 401. An `Err` here would surface as a 500 on
    /// every request carrying a stale cookie.
    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn an_unknown_session_loads_as_none_rather_than_an_error() {
        let pool = pool().await;

        match load_session(&unique_session(), pool).await {
            Ok(None) => {}
            other => panic!("expected Ok(None) for a session that was never stored, got {other:?}"),
        }
    }

    /// The sliding half of the expiry model: touching a session has to push the
    /// key's deadline back out, or the idle timeout is really an absolute one.
    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn touching_a_session_pushes_the_idle_deadline_back_out() {
        let short = Duration::from_secs(5);
        let pool = pool().await;
        let (user_id, session_id) = (unique_user(), unique_session());
        let mut session = session(user_id, ABSOLUTE);

        if let Err(error) =
            create_session(&session_id, &session, short, ABSOLUTE, pool.clone()).await
        {
            panic!("create_session: {error}");
        }

        tokio::time::sleep(Duration::from_millis(2100)).await;
        let before = ttl_of(&pool, &session_key(&session_id.0)).await;

        session.touch(chrono::Utc::now());
        if let Err(error) = touch_session(&session_id, &session, short, pool.clone()).await {
            panic!("touch_session: {error}");
        }
        let after = ttl_of(&pool, &session_key(&session_id.0)).await;

        assert!(before <= 3, "TTL {before}s did not decay while we waited");
        assert!(
            after > before,
            "the touch did not extend the session: TTL went {before}s -> {after}s"
        );
    }

    /// Logging out one device is not logging out the account.
    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn deleting_one_session_leaves_the_users_others_alone() {
        let pool = pool().await;
        let user_id = unique_user();
        let (this_device, other_device) = (unique_session(), unique_session());

        store(&pool, &this_device, user_id).await;
        store(&pool, &other_device, user_id).await;

        if let Err(error) = delete_session(&this_device, user_id, pool.clone()).await {
            panic!("delete_session: {error}");
        }

        assert!(!record_exists(&pool, &this_device).await);
        assert!(!is_indexed(&pool, user_id, &this_device).await);
        assert!(record_exists(&pool, &other_device).await);
        assert!(is_indexed(&pool, user_id, &other_device).await);
    }

    /// `SREM key` with no members is an error in Valkey, not a no-op, so the
    /// empty case needs its early return. Without it, "log out everywhere" on an
    /// account with nothing to revoke answers 500.
    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn revoking_all_sessions_with_nothing_to_revoke_succeeds() {
        let pool = pool().await;

        match delete_all_sessions(unique_user(), pool).await {
            Ok(0) => {}
            other => panic!("expected Ok(0) for a user with no sessions, got {other:?}"),
        }
    }

    #[tokio::test]
    #[ignore = "needs a live Valkey"]
    async fn revoking_all_sessions_clears_every_record_and_index_entry() {
        let pool = pool().await;
        let user_id = unique_user();
        let devices = [unique_session(), unique_session(), unique_session()];

        for session_id in &devices {
            store(&pool, session_id, user_id).await;
        }

        match delete_all_sessions(user_id, pool.clone()).await {
            Ok(3) => {}
            other => panic!("expected Ok(3) for three revoked sessions, got {other:?}"),
        }

        for session_id in &devices {
            assert!(!record_exists(&pool, session_id).await);
            assert!(!is_indexed(&pool, user_id, session_id).await);
        }
    }

    /// The invariant `delete_all_sessions` is written around: it `SREM`s exactly
    /// the ids it read instead of `DEL`ing the index key. A login landing
    /// between the read and the write keeps its index entry; under a `DEL` that
    /// session survives as a *record with no index entry* — still a working
    /// credential, and invisible to every future revoke-all.
    ///
    /// This is a probe, not a proof. It drives the two writers at each other and
    /// checks the orphan never appears: a failure here is real, a pass is
    /// evidence.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "needs a live Valkey"]
    async fn a_login_landing_mid_revoke_is_never_orphaned() {
        let pool = pool().await;

        for round in 0..25 {
            let user_id = unique_user();
            let (existing, arriving) = (unique_session(), unique_session());
            store(&pool, &existing, user_id).await;

            let session = session(user_id, ABSOLUTE);
            let (revoked, created) = tokio::join!(
                delete_all_sessions(user_id, pool.clone()),
                create_session(&arriving, &session, IDLE, ABSOLUTE, pool.clone()),
            );

            if let Err(error) = revoked {
                panic!("round {round}: delete_all_sessions: {error}");
            }
            if let Err(error) = created {
                panic!("round {round}: create_session: {error}");
            }

            let alive = record_exists(&pool, &arriving).await;
            let indexed = is_indexed(&pool, user_id, &arriving).await;

            assert!(
                !(alive && !indexed),
                "round {round}: the arriving session is alive but missing from the index — \
                 it can never be bulk-revoked again"
            );
        }
    }
}
