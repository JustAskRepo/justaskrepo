use std::sync::Arc;

use crate::{
    infrastructure::context::AuthContext, modules::auth::api::CompleteGithubLoginCommand,
    shared_kernel::error::AppError,
};

use crate::modules::auth::{
    infrastructure::github_oauth_client, infrastructure::oauth_state_store,
};

pub(crate) async fn run(
    cmd: CompleteGithubLoginCommand,
    auth: Arc<AuthContext>,
    valkey: deadpool_redis::Pool,
    http: reqwest::Client,
) -> Result<String, AppError> {
    oauth_state_store::verify_and_consume_state_valkey(&cmd.state, valkey).await?;
    let token = github_oauth_client::exchange_code_for_token(&cmd.code, &auth, &http).await?;
    let user_data = github_oauth_client::fetch_user_profile(&token, &http).await?;
    // Ok(redirect_url)
    todo!()
}
