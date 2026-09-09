use chrono::{DateTime, Utc};
use secrecy::SecretString;

use crate::shared_kernel::types::{InstallationId, RepoFullName, RepositoryId};

/// Whether a grant covers every repository on the account or a chosen subset.
///
/// Load-bearing rather than descriptive: under `All`, GitHub emits no delta
/// webhook for repositories created afterwards, so a stored set can be short
/// with no event ever saying so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::modules::installations) enum RepositorySelection {
    All,
    Selected,
}

impl RepositorySelection {
    /// The stored form, matched by the CHECK constraint in migration 0002.
    pub(in crate::modules::installations) fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Selected => "selected",
        }
    }

    /// Anything unrecognised is treated as `Selected`, the conservative reading:
    /// it says "the stored set is what you have" rather than inviting a caller
    /// to assume more repositories exist than we know about.
    pub(in crate::modules::installations) fn from_github(raw: &str) -> Self {
        match raw {
            "all" => Self::All,
            _ => Self::Selected,
        }
    }
}

/// A GitHub App installation — the grant itself, as GitHub describes it.
#[derive(Debug)]
pub(in crate::modules::installations) struct Installation {
    pub(in crate::modules::installations) id: InstallationId,
    /// Plain `i64` and not `GitHubId`: for an organization installation this is
    /// an org id, and typing it as a user id is the exact confusion ADR-009
    /// rejected option (c) for.
    pub(in crate::modules::installations) account_id: i64,
    pub(in crate::modules::installations) account_login: String,
    pub(in crate::modules::installations) account_type: String,
    pub(in crate::modules::installations) repository_selection: RepositorySelection,
    pub(in crate::modules::installations) suspended_at: Option<DateTime<Utc>>,
}

/// A repository reachable through some installation.
#[derive(Debug)]
pub(in crate::modules::installations) struct Repository {
    pub(in crate::modules::installations) id: RepositoryId,
    pub(in crate::modules::installations) installation_id: InstallationId,
    pub(in crate::modules::installations) full_name: RepoFullName,
    pub(in crate::modules::installations) private: bool,
    pub(in crate::modules::installations) default_branch: String,
}

/// A short-lived credential for reading one installation's repositories.
///
/// No `Serialize`: `secrecy` withholds it on purpose so a secret cannot be
/// written out by accident. The cache exposes the inner string explicitly at
/// its own boundary, which is where that decision belongs.
#[derive(Debug)]
pub(in crate::modules::installations) struct InstallationToken {
    pub(in crate::modules::installations) token: SecretString,
    pub(in crate::modules::installations) expires_at: DateTime<Utc>,
}
