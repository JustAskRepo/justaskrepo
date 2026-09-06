use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::modules::auth::domain::session::RevocationScope;
use crate::shared_kernel::{
    domain_events::DomainEvent,
    types::{CorrelationId, GitHubId, UserId},
};

/// A user completed the GitHub handshake and now holds a live session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAuthenticatedEvent {
    pub user_id: UserId,
    /// Carried so a subscriber can talk to GitHub about this user without
    /// asking `auth` for the mapping first.
    pub github_id: GitHubId,
    pub occurred_at: DateTime<Utc>,
    pub correlation_id: CorrelationId,
}

impl UserAuthenticatedEvent {
    #[must_use]
    pub fn new(user_id: UserId, github_id: GitHubId, now: DateTime<Utc>) -> Self {
        Self {
            user_id,
            github_id,
            occurred_at: now,
            correlation_id: CorrelationId::new(),
        }
    }
}

impl DomainEvent for UserAuthenticatedEvent {
    fn event_name(&self) -> &'static str {
        "auth.user_authenticated"
    }

    fn correlation_id(&self) -> CorrelationId {
        self.correlation_id
    }
}

/// Sessions belonging to a user stopped working. One event for both revocation
/// commands, because a subscriber cares about the same thing either way — this
/// user's credentials changed — and `scope` is what separates a single sign-out
/// from a sign-out everywhere.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSessionsRevokedEvent {
    pub user_id: UserId,
    pub scope: RevocationScope,
    pub occurred_at: DateTime<Utc>,
    pub correlation_id: CorrelationId,
}

impl UserSessionsRevokedEvent {
    #[must_use]
    pub fn new(user_id: UserId, scope: RevocationScope, now: DateTime<Utc>) -> Self {
        Self {
            user_id,
            scope,
            occurred_at: now,
            correlation_id: CorrelationId::new(),
        }
    }
}

impl DomainEvent for UserSessionsRevokedEvent {
    fn event_name(&self) -> &'static str {
        "auth.user_sessions_revoked"
    }

    fn correlation_id(&self) -> CorrelationId {
        self.correlation_id
    }
}
