use std::sync::Arc;

use crate::{
    infrastructure::context::AuthContext, modules::auth::api::StartGithubLoginCommand,
    shared_kernel::error::AppError,
};

use crate::modules::auth::domain::generate_state;
use crate::modules::auth::{
    infrastructure::github_oauth_client, infrastructure::oauth_state_store,
};

pub(crate) async fn run(
    _cmd: StartGithubLoginCommand,
    auth: Arc<AuthContext>,
    valkey: deadpool_redis::Pool,
) -> Result<String, AppError> {
    let state = generate_state::generate_state();
    oauth_state_store::store_state_valkey(&state, valkey.clone()).await?;
    let authorize_url = github_oauth_client::get_authorize_url(&state, &auth)?;
    Ok(authorize_url)
}
