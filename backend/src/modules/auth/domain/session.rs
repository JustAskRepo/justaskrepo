use std::{ops::Not, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::shared_kernel::{
    error::AppError,
    types::{GitHubId, UserId},
};

#[derive(Debug, Serialize, Deserialize)]
pub(in crate::modules::auth) struct Session {
    pub(in crate::modules::auth) user_id: UserId,
    pub(in crate::modules::auth) github_id: GitHubId,
    pub(in crate::modules::auth) created_at: DateTime<Utc>,
    pub(in crate::modules::auth) last_seen_at: DateTime<Utc>,
    pub(in crate::modules::auth) expires_at: DateTime<Utc>,
    pub(in crate::modules::auth) ip: String,
    pub(in crate::modules::auth) user_agent: String,
}

impl Session {
    pub(in crate::modules::auth) fn new(
        user_id: UserId,
        github_id: GitHubId,
        ip: String,
        user_agent: String,
        now: DateTime<Utc>,
        absolute_ttl: Duration,
    ) -> Result<Self, AppError> {
        let ttl = TimeDelta::from_std(absolute_ttl).map_err(AppError::internal)?;
        let expires_at = now
            .checked_add_signed(ttl)
            .ok_or_else(|| AppError::Validation("session absolute TTL overflows".to_owned()))?;

        Ok(Self {
            user_id,
            github_id,
            created_at: now,
            last_seen_at: now,
            expires_at,
            ip,
            user_agent,
        })
    }

    /// The 30-day absolute cap. Valkey's TTL is the sliding idle timeout and is
    /// pushed forward on activity, so it can never enforce this — every load has
    /// to check it explicitly.
    pub(in crate::modules::auth) fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    pub(in crate::modules::auth) fn needs_refresh(
        &self,
        now: DateTime<Utc>,
        threshold: Duration,
    ) -> bool {
        TimeDelta::from_std(threshold)
            .is_ok_and(|threshold| now - self.last_seen_at < threshold)
            .not()
    }

    pub(in crate::modules::auth) fn touch(&mut self, now: DateTime<Utc>) {
        self.last_seen_at = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABSOLUTE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
    const THRESHOLD: Duration = Duration::from_secs(300);

    fn session(now: DateTime<Utc>) -> Session {
        match Session::new(
            UserId(1),
            GitHubId(2),
            "203.0.113.7".to_owned(),
            "probe".to_owned(),
            now,
            ABSOLUTE,
        ) {
            Ok(session) => session,
            Err(error) => panic!("a 30-day TTL is representable: {error}"),
        }
    }

    fn after(instant: DateTime<Utc>, seconds: i64) -> DateTime<Utc> {
        instant + TimeDelta::seconds(seconds)
    }

    #[test]
    fn a_new_session_starts_both_clocks_together_and_caps_at_the_ttl() {
        let now = Utc::now();
        let session = session(now);

        assert_eq!(session.created_at, now);
        assert_eq!(session.last_seen_at, now);
        assert_eq!(session.expires_at, now + TimeDelta::days(30));
    }

    /// `>=`, not `>`. The cap is the first instant the session is dead, and an
    /// off-by-one here is a credential that outlives its own expiry.
    #[test]
    fn the_cap_is_the_first_expired_instant() {
        let session = session(Utc::now());

        assert!(!session.is_expired(after(session.expires_at, -1)));
        assert!(session.is_expired(session.expires_at));
        assert!(session.is_expired(after(session.expires_at, 1)));
    }

    /// The reason `expires_at` is carried next to a Valkey TTL at all: activity
    /// slides the idle timeout and nothing slides the absolute cap. The day
    /// `touch` starts moving `expires_at`, a session used daily lives forever
    /// and the 30-day cap quietly stops existing.
    #[test]
    fn activity_moves_last_seen_but_never_the_cap() {
        let now = Utc::now();
        let mut session = session(now);
        let cap = session.expires_at;

        session.touch(after(now, 3600));

        assert_eq!(session.last_seen_at, after(now, 3600));
        assert_eq!(session.expires_at, cap);
    }

    /// The throttle that keeps an authenticated request from costing a Valkey
    /// write. Refresh *at* the threshold, never before it.
    #[test]
    fn a_session_is_refreshed_only_once_the_threshold_has_elapsed() {
        let now = Utc::now();
        let session = session(now);

        assert!(!session.needs_refresh(now, THRESHOLD));
        assert!(!session.needs_refresh(after(now, 299), THRESHOLD));
        assert!(session.needs_refresh(after(now, 300), THRESHOLD));
        assert!(session.needs_refresh(after(now, 3600), THRESHOLD));
    }

    /// `Duration::MAX` does not fit in a `TimeDelta`. The arithmetic has to
    /// answer with an error rather than panicking or wrapping — a wrapped cap
    /// is a session that is born expired, or worse, born eternal.
    #[test]
    fn an_unrepresentable_ttl_is_an_error_not_a_panic() {
        let result = Session::new(
            UserId(1),
            GitHubId(2),
            "203.0.113.7".to_owned(),
            "probe".to_owned(),
            Utc::now(),
            Duration::MAX,
        );

        assert!(result.is_err());
    }
}
