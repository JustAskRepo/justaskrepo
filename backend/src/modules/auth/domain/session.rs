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
