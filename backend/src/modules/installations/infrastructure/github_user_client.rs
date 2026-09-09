// modules/installations/infrastructure/github_user_client.rs
//
// The *user*-token half of GitHub. Kept apart from github_app_client.rs on
// purpose: the two credentials answer different questions — "which installations
// may this human see" versus "may we read this repository" — and merging the
// callers of both into one file is how that distinction gets lost.

use secrecy::SecretString;

use crate::modules::installations::domain::installation::{Installation, Repository};
use crate::modules::installations::infrastructure::github_api::{
    GITHUB_API, InstallationDto, InstallationsPage, MAX_PAGES, PER_PAGE, RepositoriesPage,
    RepositoryDto, get_json,
};
use crate::shared_kernel::{error::AppError, types::InstallationId};

/// Which installations this human may see, straight from GitHub.
///
/// This is the authoritative answer ADR-009 spends the user token on, and the
/// only moment we can ask it — the token is gone by the next request.
#[tracing::instrument(skip_all, err)]
pub(in crate::modules::installations) async fn list_user_installations(
    token: &SecretString,
    http: &reqwest::Client,
) -> Result<Vec<Installation>, AppError> {
    let mut out = Vec::new();

    for page in 1..=MAX_PAGES {
        let url = format!("{GITHUB_API}/user/installations?per_page={PER_PAGE}&page={page}");
        let body: InstallationsPage = get_json(&url, token, http).await?;
        let count = body.installations.len();

        out.extend(
            body.installations
                .into_iter()
                .map(InstallationDto::into_domain),
        );

        if count < PER_PAGE {
            return Ok(out);
        }
    }

    tracing::warn!(
        max_pages = MAX_PAGES,
        "installation listing hit the page cap; the tail was not read"
    );
    Ok(out)
}

/// The repositories *this user* may reach through one installation.
///
/// Deliberately not `GET /installation/repositories`, which answers the wider
/// question of what the installation covers. For an organization the two differ,
/// and showing a member the wider set is an over-grant (migration 0003).
#[tracing::instrument(skip_all, err)]
pub(in crate::modules::installations) async fn list_user_repositories(
    installation_id: InstallationId,
    token: &SecretString,
    http: &reqwest::Client,
) -> Result<Vec<Repository>, AppError> {
    let mut out = Vec::new();

    for page in 1..=MAX_PAGES {
        let url = format!(
            "{GITHUB_API}/user/installations/{}/repositories?per_page={PER_PAGE}&page={page}",
            installation_id.0
        );
        let body: RepositoriesPage = get_json(&url, token, http).await?;
        let count = body.repositories.len();

        out.extend(
            body.repositories
                .into_iter()
                .map(|repo| RepositoryDto::into_domain(repo, installation_id)),
        );

        if count < PER_PAGE {
            return Ok(out);
        }
    }

    tracing::warn!(
        installation_id = installation_id.0,
        max_pages = MAX_PAGES,
        "repository listing hit the page cap; the tail was not read"
    );
    Ok(out)
}
