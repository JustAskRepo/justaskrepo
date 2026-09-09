use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::shared_kernel::{
    domain_events::DomainEvent,
    types::{CorrelationId, InstallationId, RepoFullName, RepositoryId, UserId},
};

/// A repository became readable by the App.
///
/// One event per repository rather than one per installation, because the
/// subscriber that matters is `indexing` and a job is per repository.
/// Installing on fifty repositories publishes fifty of these.
///
/// It carries `repo_full_name` despite the ARCHITECTURE.md §6 rule that events
/// hold only what a subscriber could not fetch itself: a numeric id cannot be
/// cloned. Withholding it would make every subscriber open with the same query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInstalledEvent {
    pub installation_id: InstallationId,
    pub repository_id: RepositoryId,
    pub repo_full_name: RepoFullName,
    pub occurred_at: DateTime<Utc>,
    pub correlation_id: CorrelationId,
}

impl RepoInstalledEvent {
    #[must_use]
    pub fn new(
        installation_id: InstallationId,
        repository_id: RepositoryId,
        repo_full_name: RepoFullName,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            installation_id,
            repository_id,
            repo_full_name,
            occurred_at: now,
            correlation_id: CorrelationId::new(),
        }
    }
}

impl DomainEvent for RepoInstalledEvent {
    fn event_name(&self) -> &'static str {
        "installations.repo_installed"
    }

    fn correlation_id(&self) -> CorrelationId {
        self.correlation_id
    }
}

/// An installation was removed, and one user's access went with it.
///
/// `user_id` is singular and the publisher fans out. An organization
/// installation can be linked to many users, and the reaction waiting for this
/// is `auth`'s `RevokeAllSessionsCommand`, which takes exactly one user — so one
/// event per affected user keeps that handler a straight mapping rather than a
/// loop over a payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoUninstalledEvent {
    pub installation_id: InstallationId,
    pub user_id: UserId,
    pub occurred_at: DateTime<Utc>,
    pub correlation_id: CorrelationId,
}

impl RepoUninstalledEvent {
    #[must_use]
    pub fn new(installation_id: InstallationId, user_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            installation_id,
            user_id,
            occurred_at: now,
            correlation_id: CorrelationId::new(),
        }
    }
}

impl DomainEvent for RepoUninstalledEvent {
    fn event_name(&self) -> &'static str {
        "installations.repo_uninstalled"
    }

    fn correlation_id(&self) -> CorrelationId {
        self.correlation_id
    }
}
