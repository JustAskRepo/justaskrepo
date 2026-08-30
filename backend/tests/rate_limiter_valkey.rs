// tests/rate_limiter_valkey.rs
//
// The half of the limiter unit tests cannot reach: the Valkey command sequence
// itself. `RateLimitPolicy::decide` is pure and covered in-crate; what needs a
// live server is whether `INCR` + `EXPIRE NX` + `TTL` inside `MULTI` behaves the
// way the design assumes on Valkey 8.
//
// Ignored by default so `cargo test` stays hermetic. With the compose stack up:
//
//     cargo test --test rate_limiter_valkey -- --ignored --nocapture
//
// Reading VALKEY_URL here is not a rule-10 violation: that rule scans `src/`,
// and this is a harness locating a server, not the application configuring
// itself.

use std::time::Duration;

use deadpool_redis::redis::cmd;
use justaskrepo::infrastructure::{
    config::ValkeyConfig,
    rate_limiter::{RateLimitDecision, RateLimitPolicy, check},
    valkey::connect_valkey,
};

const WINDOW: Duration = Duration::from_secs(3);

fn policy() -> RateLimitPolicy {
    RateLimitPolicy {
        name: "test",
        max_requests: 3,
        window: WINDOW,
    }
}

fn subject() -> String {
    format!(
        "probe-{}-{:?}",
        std::process::id(),
        std::time::Instant::now()
    )
}

async fn pool() -> deadpool_redis::Pool {
    let url = std::env::var("VALKEY_URL").unwrap_or_else(|_| "redis://localhost:6379".to_owned());
    let config = ValkeyConfig {
        url: url.into(),
        pool_size: 2,
    };
    match connect_valkey(&config).await {
        Ok(pool) => pool,
        Err(error) => panic!("no Valkey at VALKEY_URL — start the compose stack: {error:#}"),
    }
}

#[tokio::test]
#[ignore = "needs a live Valkey"]
async fn the_budget_is_spent_then_refused() {
    let (pool, subject, policy) = (pool().await, subject(), policy());

    for attempt in 1..=policy.max_requests {
        assert_eq!(
            check(&policy, &subject, &pool).await,
            RateLimitDecision::Allowed,
            "request {attempt} of a budget of {} should be allowed",
            policy.max_requests
        );
    }

    match check(&policy, &subject, &pool).await {
        RateLimitDecision::Allowed => panic!("the request past the budget was allowed"),
        RateLimitDecision::Exceeded { retry_after_secs } => {
            assert!(
                (1..=WINDOW.as_secs()).contains(&retry_after_secs),
                "Retry-After {retry_after_secs}s is outside the {WINDOW:?} window"
            );
        }
    }
}

#[tokio::test]
#[ignore = "needs a live Valkey"]
async fn hits_inside_the_window_do_not_extend_it() {
    let (pool, subject, policy) = (pool().await, subject(), policy());

    let _ = check(&policy, &subject, &pool).await;
    let ttl_after_first = ttl_of(&pool, &subject).await;

    tokio::time::sleep(Duration::from_millis(1500)).await;
    let _ = check(&policy, &subject, &pool).await;
    let ttl_after_second = ttl_of(&pool, &subject).await;

    assert!(
        ttl_after_second < ttl_after_first,
        "the second hit reset the window: TTL went {ttl_after_first}s -> \
         {ttl_after_second}s, so EXPIRE is not being applied with NX"
    );

    // And the window really does close, refilling the budget.
    tokio::time::sleep(WINDOW).await;
    assert_eq!(
        check(&policy, &subject, &pool).await,
        RateLimitDecision::Allowed,
        "the budget did not refill after the window elapsed"
    );
}

async fn ttl_of(pool: &deadpool_redis::Pool, subject: &str) -> i64 {
    let mut conn = match pool.get().await {
        Ok(conn) => conn,
        Err(error) => panic!("lost the Valkey connection: {error}"),
    };
    cmd("TTL")
        .arg(format!("ratelimit:test:{subject}"))
        .query_async(&mut conn)
        .await
        .unwrap_or(-2)
}
