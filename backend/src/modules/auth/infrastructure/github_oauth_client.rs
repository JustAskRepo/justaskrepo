// modules/auth/infrastructure/github_client.rs
use reqwest::{StatusCode, header};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use url::Url;

use crate::infrastructure::context::AuthContext;
use crate::modules::auth::domain::github_profile::GitHubProfile;
use crate::shared_kernel::{error::AppError, types::GitHubId};

const GITHUB_API: &str = "https://api.github.com";
const GITHUB_OAUTH: &str = "https://github.com/login/oauth";
const API_VERSION: &str = "2026-03-10";
const ACCEPT_JSON: &str = "application/vnd.github+json";

pub(in crate::modules::auth) fn build_authorize_url(
    state: &str,
    auth: &AuthContext,
) -> Result<String, AppError> {
    let mut url = Url::parse(&format!("{GITHUB_OAUTH}/authorize")).map_err(AppError::internal)?;
    url.query_pairs_mut()
        .append_pair("client_id", &auth.client_id)
        .append_pair("redirect_uri", &auth.redirect_uri)
        .append_pair("state", state);
    Ok(url.into())
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TokenExchangeDto {
    Success {
        access_token: String,
    },
    Failure {
        error: String,
        error_description: Option<String>,
    },
}

#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn exchange_code_for_token(
    code: &str,
    auth: &AuthContext,
    http: &reqwest::Client,
) -> Result<SecretString, AppError> {
    let params = [
        ("client_id", auth.client_id.as_str()),
        ("client_secret", auth.client_secret.expose_secret()),
        ("code", code),
        ("redirect_uri", auth.redirect_uri.as_str()),
    ];

    let res = http
        .post(format!("{GITHUB_OAUTH}/access_token"))
        .header(header::ACCEPT, "application/json")
        .form(&params)
        .send()
        .await
        .map_err(AppError::internal)?;

    if !res.status().is_success() {
        tracing::warn!(status = %res.status(), "unexpected status from token exchange");
        return Err(AppError::Unavailable { service: "github" });
    }

    match res
        .json::<TokenExchangeDto>()
        .await
        .map_err(AppError::internal)?
    {
        TokenExchangeDto::Success { access_token } => Ok(SecretString::from(access_token)),
        TokenExchangeDto::Failure {
            error,
            error_description,
        } => {
            tracing::warn!(%error, ?error_description, "github rejected the code exchange");
            Err(AppError::Unauthorized)
        }
    }
}

#[derive(Deserialize)]
struct GitHubUserDto {
    id: i64,
    login: String,
    name: Option<String>,
    email: Option<String>,
    avatar_url: String,
}

impl GitHubUserDto {
    fn into_domain(self) -> GitHubProfile {
        GitHubProfile {
            github_user_id: GitHubId(self.id),
            login: self.login,
            name: self.name,
            email: self.email,
            avatar_url: self.avatar_url,
        }
    }
}

#[tracing::instrument(skip_all, err)]
pub(in crate::modules::auth) async fn fetch_user_profile(
    token: &SecretString,
    http: &reqwest::Client,
) -> Result<GitHubProfile, AppError> {
    let res = http
        .get(format!("{GITHUB_API}/user"))
        .header(header::ACCEPT, ACCEPT_JSON)
        .header("X-GitHub-Api-Version", API_VERSION)
        .bearer_auth(token.expose_secret())
        .send()
        .await
        .map_err(AppError::internal)?;

    match res.status() {
        StatusCode::OK => Ok(res
            .json::<GitHubUserDto>()
            .await
            .map_err(AppError::internal)?
            .into_domain()),
        StatusCode::UNAUTHORIZED => Err(AppError::Unauthorized),
        status => {
            tracing::warn!(%status, "unexpected status from GET /user");
            Err(AppError::Unavailable { service: "github" })
        }
    }
}
