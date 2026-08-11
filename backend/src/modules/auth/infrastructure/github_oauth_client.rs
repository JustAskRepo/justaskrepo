use url::Url;

use crate::{infrastructure::context::AuthContext, shared_kernel::error::AppError};

pub fn get_authorize_url(state: &str, auth: &AuthContext) -> Result<String, AppError> {
    let mut url =
        Url::parse("https://github.com/login/oauth/authorize").map_err(AppError::internal)?;
    url.query_pairs_mut()
        .append_pair("client_id", &auth.client_id)
        .append_pair("redirect_uri", &auth.redirect_uri)
        .append_pair("state", state);
    Ok(url.into())
}
