// modules/installations/infrastructure/github_app_client.rs
//
// The *app* half of GitHub: credentials that represent the installation grant
// rather than a person. Kept apart from github_user_client.rs because confusing
// the two is the standard way to build this module wrong — a user token can
// never read a repository here, and an installation token can never tell you
// which installations a human may see.

use chrono::{DateTime, TimeDelta, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::{StatusCode, header};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::modules::installations::domain::installation::{
    Installation, InstallationToken, Repository,
};
use crate::modules::installations::infrastructure::github_api::{
    GITHUB_API, InstallationDto, MAX_PAGES, PER_PAGE, RepositoriesPage, RepositoryDto, get_json,
};
use crate::shared_kernel::{error::AppError, types::InstallationId};

const GITHUB: &str = "https://github.com";
const API_VERSION: &str = "2026-03-10";
const ACCEPT_JSON: &str = "application/vnd.github+json";

/// GitHub rejects a JWT whose `iat` is in its future, and clocks disagree. The
/// documented remedy is to backdate the issue time.
const CLOCK_SKEW_ALLOWANCE: TimeDelta = TimeDelta::seconds(60);

/// GitHub caps App JWT lifetime at ten minutes and rejects anything longer.
/// Nine leaves room for the backdating above to be counted against it.
const APP_JWT_LIFETIME: TimeDelta = TimeDelta::minutes(9);

/// Where a user goes to install the App and pick repositories.
///
/// Infallible rather than `Result`: `config.rs` rejects a slug that is not
/// bare ASCII at startup, so by the time one reaches here there is nothing left
/// to fail on. Validating twice would only invent an error path no caller can
/// trigger.
pub(in crate::modules::installations) fn build_install_url(app_slug: &str) -> String {
    format!("{GITHUB}/apps/{app_slug}/installations/new")
}

#[derive(Serialize)]
struct AppJwtClaims {
    iat: i64,
    exp: i64,
    iss: String,
}

/// Signs a JWT that authenticates *the app itself*. It proves nothing about any
/// user and grants no repository access — its only use is exchanging it for an
/// installation token below.
///
/// Never returned from this module and never logged: it is the App private key
/// in a portable wrapper, and anyone holding one can mint a token for every
/// installation the App has.
fn mint_app_jwt(app_id: u64, private_key_pem: &SecretString) -> Result<SecretString, AppError> {
    let now = Utc::now();
    let claims = AppJwtClaims {
        iat: (now - CLOCK_SKEW_ALLOWANCE).timestamp(),
        exp: (now + APP_JWT_LIFETIME).timestamp(),
        iss: app_id.to_string(),
    };

    let key =
        EncodingKey::from_rsa_pem(private_key_pem.expose_secret().as_bytes()).map_err(|error| {
            // The PEM is operator config, so a failure here is a deployment
            // fault rather than a runtime one — say so plainly in the log.
            tracing::error!(%error, "the GitHub App private key is not a usable RSA PEM");
            AppError::internal(error)
        })?;

    encode(&Header::new(Algorithm::RS256), &claims, &key)
        .map(SecretString::from)
        .map_err(AppError::internal)
}

#[derive(Deserialize)]
struct InstallationTokenDto {
    token: String,
    expires_at: DateTime<Utc>,
}

/// Exchanges an App JWT for a token scoped to one installation.
///
/// GitHub gives these an hour and will not refresh them — the answer to an
/// expiring token is another token, which is why the cache above it stops
/// serving one well before it dies.
#[tracing::instrument(skip_all, fields(installation_id = installation_id.0), err)]
pub(in crate::modules::installations) async fn mint_installation_token(
    installation_id: InstallationId,
    app_id: u64,
    private_key_pem: &SecretString,
    http: &reqwest::Client,
) -> Result<InstallationToken, AppError> {
    let app_jwt = mint_app_jwt(app_id, private_key_pem)?;

    let res = http
        .post(format!(
            "{GITHUB_API}/app/installations/{}/access_tokens",
            installation_id.0
        ))
        .header(header::ACCEPT, ACCEPT_JSON)
        .header("X-GitHub-Api-Version", API_VERSION)
        .bearer_auth(app_jwt.expose_secret())
        .send()
        .await
        .map_err(AppError::internal)?;

    match res.status() {
        StatusCode::CREATED | StatusCode::OK => {
            let dto: InstallationTokenDto = res.json().await.map_err(AppError::internal)?;
            Ok(InstallationToken {
                token: SecretString::from(dto.token),
                expires_at: dto.expires_at,
            })
        }
        // The App was uninstalled, or the id was never ours. Either way this is
        // not a transient failure and a retry will not help.
        StatusCode::NOT_FOUND => Err(AppError::NotFound {
            resource: "installation",
        }),
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            tracing::error!(
                status = %res.status(),
                "github refused the App JWT — check GITHUB_APP_ID and the private key"
            );
            Err(AppError::Unauthorized)
        }
        status => {
            tracing::warn!(%status, "unexpected status minting an installation token");
            Err(AppError::Unavailable { service: "github" })
        }
    }
}

/// The installation record as GitHub currently describes it.
///
/// The webhook that triggers a sync carries a payload saying the same thing,
/// and we ask anyway. Deliveries are at-least-once and unordered, so a payload
/// is a claim about a moment that may already have passed; this is the state
/// now. Trading one request for immunity to an entire class of ordering bug is
/// the cheap side of that bargain (guide §8).
#[tracing::instrument(skip_all, fields(installation_id = installation_id.0), err)]
pub(in crate::modules::installations) async fn fetch_installation(
    installation_id: InstallationId,
    app_id: u64,
    private_key_pem: &SecretString,
    http: &reqwest::Client,
) -> Result<Installation, AppError> {
    let app_jwt = mint_app_jwt(app_id, private_key_pem)?;
    let url = format!("{GITHUB_API}/app/installations/{}", installation_id.0);
    let dto: InstallationDto = get_json(&url, &app_jwt, http).await?;
    Ok(dto.into_domain())
}

/// Every repository the *installation* covers — the wider, app-scoped set.
///
/// This is what `installation_repositories` stores, and it is not what a given
/// organization member may see. `github_user_client::list_user_repositories`
/// answers that narrower question, and authorization uses only that one.
#[tracing::instrument(skip_all, fields(installation_id = installation_id.0), err)]
pub(in crate::modules::installations) async fn list_installation_repositories(
    installation_id: InstallationId,
    installation_token: &SecretString,
    http: &reqwest::Client,
) -> Result<Vec<Repository>, AppError> {
    let mut out = Vec::new();

    for page in 1..=MAX_PAGES {
        let url = format!("{GITHUB_API}/installation/repositories?per_page={PER_PAGE}&page={page}");
        let body: RepositoriesPage = get_json(&url, installation_token, http).await?;
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
        "installation repository listing hit the page cap; the tail was not read"
    );
    Ok(out)
}
