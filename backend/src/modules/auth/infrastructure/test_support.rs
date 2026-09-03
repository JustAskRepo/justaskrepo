// modules/auth/infrastructure/test_support.rs
//
// Valkey plumbing shared by this module's `#[ignore]`d integration tests.
//
// Those tests live in-crate rather than in `tests/` for a boring reason: what
// they exercise is `pub(in crate::modules::auth)` and must stay that way
// (architecture rule 1). An integration test cannot reach a module's
// infrastructure layer, so the choice is between having these tests and having
// the boundary — and the boundary wins.

use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::Duration,
};

use chrono::Utc;

use crate::{
    infrastructure::{config::ValkeyConfig, valkey::connect_valkey},
    modules::auth::domain::session::Session,
    shared_kernel::types::{GitHubId, SessionId, UserId},
};

/// Architecture rule 10 keeps `env::var` out of `src/` entirely, tests included,
/// so this is the compose default rather than `VALKEY_URL`. Moving Valkey means
/// editing this line — the correct trade for one door to the environment.
const VALKEY_URL: &str = "redis://localhost:6379";

/// Four connections, not one: the revoke-all race probe needs two writers
/// genuinely in flight at the same time.
pub(in crate::modules::auth) async fn pool() -> deadpool_redis::Pool {
    let config = ValkeyConfig {
        url: VALKEY_URL.to_owned().into(),
        pool_size: 4,
    };

    match connect_valkey(&config).await {
        Ok(pool) => pool,
        Err(error) => panic!("no Valkey at {VALKEY_URL} — start the compose stack: {error:#}"),
    }
}

static NEXT: AtomicI64 = AtomicI64::new(0);

fn next() -> i64 {
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Subjects are unique per test, per process. Tests share one server and run in
/// threads; a fixed user id would have them revoking each other's sessions.
pub(in crate::modules::auth) fn unique_user() -> UserId {
    UserId(i64::from(std::process::id()) * 1_000_000 + next())
}

pub(in crate::modules::auth) fn unique_token() -> String {
    format!("probe-{}-{}", std::process::id(), next())
}

pub(in crate::modules::auth) fn unique_session() -> SessionId {
    SessionId(unique_token())
}

/// A session for `user_id` whose absolute cap is `absolute_ttl` from now. Pass
/// `Duration::ZERO` for one that is already past its cap.
pub(in crate::modules::auth) fn session(user_id: UserId, absolute_ttl: Duration) -> Session {
    match Session::new(
        user_id,
        GitHubId(user_id.0),
        "203.0.113.7".to_owned(),
        "probe".to_owned(),
        Utc::now(),
        absolute_ttl,
    ) {
        Ok(session) => session,
        Err(error) => panic!("building a test session: {error}"),
    }
}
