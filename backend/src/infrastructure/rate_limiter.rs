// infrastructure/rate_limiter.rs
//
// A fixed-window counter in Valkey, shared by every rate-limited entry point.
//
// It sits beside db.rs and valkey.rs rather than inside infrastructure/http/
// because a budget is not an HTTP concern. The WebSocket ticket issuer and any
// future background trigger want the same counter; only the layer above knows
// how to turn a refusal into a 429.
//
// Fixed window, not a sliding log: a caller straddling a boundary can spend up
// to 2x the budget across two adjacent windows. At these budgets that is 40
// requests instead of 20 — irrelevant against the abuse this protects — and it
// costs one integer key per subject instead of one sorted-set member per
// request. Revisit only if a budget gets small enough for the doubling to bite.

use std::time::Duration;

use deadpool_redis::{Pool, redis::pipe};

#[derive(Debug, Clone, Copy)]
pub struct RateLimitPolicy {
    pub name: &'static str,
    pub max_requests: u32,
    pub window: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum RateLimitDecision {
    Allowed,
    Exceeded { retry_after_secs: u64 },
}

#[tracing::instrument(skip(policy, valkey), fields(policy = policy.name))]
pub async fn check(policy: &RateLimitPolicy, subject: &str, valkey: &Pool) -> RateLimitDecision {
    match count_hit(policy, subject, valkey).await {
        Ok((used, ttl_secs)) => policy.decide(used, ttl_secs),
        Err(error) => {
            tracing::error!(
                ?error,
                policy = policy.name,
                "rate limiter unavailable — allowing the request"
            );
            RateLimitDecision::Allowed
        }
    }
}

async fn count_hit(
    policy: &RateLimitPolicy,
    subject: &str,
    valkey: &Pool,
) -> anyhow::Result<(u64, i64)> {
    let mut conn = valkey.get().await?;
    let key = format!("ratelimit:{}:{subject}", policy.name);
    let window_secs = policy.window.as_secs();

    let (used, _expire_was_set, ttl_secs): (u64, i64, i64) = pipe()
        .atomic()
        .cmd("INCR")
        .arg(&key)
        .cmd("EXPIRE")
        .arg(&key)
        .arg(window_secs)
        .arg("NX")
        .cmd("TTL")
        .arg(&key)
        .query_async(&mut conn)
        .await?;

    Ok((used, ttl_secs))
}

impl RateLimitPolicy {
    fn decide(&self, used: u64, ttl_secs: i64) -> RateLimitDecision {
        if used <= u64::from(self.max_requests) {
            return RateLimitDecision::Allowed;
        }
        let retry_after_secs = u64::try_from(ttl_secs)
            .unwrap_or(self.window.as_secs())
            .max(1);

        RateLimitDecision::Exceeded { retry_after_secs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> RateLimitPolicy {
        RateLimitPolicy {
            name: "test",
            max_requests: 3,
            window: Duration::from_secs(60),
        }
    }

    #[test]
    fn the_whole_budget_is_spendable() {
        assert_eq!(policy().decide(1, 60), RateLimitDecision::Allowed);
        assert_eq!(policy().decide(3, 58), RateLimitDecision::Allowed);
    }

    /// Off-by-one here is the difference between a budget of 3 and a budget of
    /// 4, and nothing else in the system would notice.
    #[test]
    fn the_request_after_the_budget_is_refused() {
        assert_eq!(
            policy().decide(4, 58),
            RateLimitDecision::Exceeded {
                retry_after_secs: 58
            }
        );
    }

    /// `Retry-After: 0` parses fine and tells the client to retry immediately,
    /// straight into another refusal.
    #[test]
    fn retry_after_is_never_zero() {
        assert_eq!(
            policy().decide(9, 0),
            RateLimitDecision::Exceeded {
                retry_after_secs: 1
            }
        );
    }

    /// -1 (no expiry) and -2 (no key) are both unreachable after an `INCR` in
    /// the same transaction; if Valkey ever says otherwise, fall back to the
    /// configured window rather than emitting a nonsense header.
    #[test]
    fn a_negative_ttl_falls_back_to_the_window() {
        assert_eq!(
            policy().decide(4, -1),
            RateLimitDecision::Exceeded {
                retry_after_secs: 60
            }
        );
        assert_eq!(
            policy().decide(4, -2),
            RateLimitDecision::Exceeded {
                retry_after_secs: 60
            }
        );
    }
}
