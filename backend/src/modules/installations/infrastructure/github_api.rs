// modules/installations/infrastructure/github_api.rs
//
// How we talk to GitHub's REST API, independent of *which* credential we hold.
// The two clients beside this file differ only in the credential they present
// and the endpoints they may reach; the wire shapes and the request mechanics
// are the same, and one copy of them cannot drift from the other.

use reqwest::{StatusCode, header};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use crate::modules::installations::domain::installation::{
    Installation, Repository, RepositorySelection,
};
use crate::shared_kernel::{
    error::AppError,
    types::{InstallationId, RepoFullName, RepositoryId},
};

pub(in crate::modules::installations) const GITHUB_API: &str = "https://api.github.com";
const API_VERSION: &str = "2026-03-10";
const ACCEPT_JSON: &str = "application/vnd.github+json";

pub(in crate::modules::installations) const PER_PAGE: usize = 100;

/// Ceiling on pagination. Ten pages is far past any real account, and the point
/// is that a paging bug or a surprising response shape cannot spin a request
/// forever — a bounded wrong answer beats an unbounded one.
pub(in crate::modules::installations) const MAX_PAGES: usize = 10;

#[derive(Deserialize)]
pub(in crate::modules::installations) struct AccountDto {
    id: i64,
    login: String,
    #[serde(rename = "type")]
    account_type: String,
}

#[derive(Deserialize)]
pub(in crate::modules::installations) struct InstallationDto {
    id: i64,
    account: AccountDto,
    repository_selection: String,
    suspended_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize)]
pub(in crate::modules::installations) struct InstallationsPage {
    pub(in crate::modules::installations) installations: Vec<InstallationDto>,
}

#[derive(Deserialize)]
pub(in crate::modules::installations) struct RepositoryDto {
    id: i64,
    full_name: String,
    private: bool,
    default_branch: String,
}

#[derive(Deserialize)]
pub(in crate::modules::installations) struct RepositoriesPage {
    pub(in crate::modules::installations) repositories: Vec<RepositoryDto>,
}

impl InstallationDto {
    pub(in crate::modules::installations) fn into_domain(self) -> Installation {
        Installation {
            id: InstallationId(self.id),
            account_id: self.account.id,
            account_login: self.account.login,
            account_type: self.account.account_type,
            repository_selection: RepositorySelection::from_github(&self.repository_selection),
            suspended_at: self.suspended_at,
        }
    }
}

impl RepositoryDto {
    pub(in crate::modules::installations) fn into_domain(
        self,
        installation_id: InstallationId,
    ) -> Repository {
        Repository {
            id: RepositoryId(self.id),
            installation_id,
            full_name: RepoFullName(self.full_name),
            private: self.private,
            default_branch: self.default_branch,
        }
    }
}

/// One authenticated GET, deserialized. The caller supplies the bearer token,
/// which is the only thing that differs between the app and user clients.
pub(in crate::modules::installations) async fn get_json<T: serde::de::DeserializeOwned>(
    url: &str,
    token: &SecretString,
    http: &reqwest::Client,
) -> Result<T, AppError> {
    let res = http
        .get(url)
        .header(header::ACCEPT, ACCEPT_JSON)
        .header("X-GitHub-Api-Version", API_VERSION)
        .bearer_auth(token.expose_secret())
        .send()
        .await
        .map_err(AppError::internal)?;

    match res.status() {
        StatusCode::OK => res.json::<T>().await.map_err(AppError::internal),
        StatusCode::NOT_FOUND => Err(AppError::NotFound {
            resource: "installation",
        }),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(AppError::Unauthorized),
        status => {
            tracing::warn!(%status, %url, "unexpected status from the GitHub API");
            Err(AppError::Unavailable { service: "github" })
        }
    }
}
