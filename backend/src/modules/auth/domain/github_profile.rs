use crate::shared_kernel::types::GitHubId;
pub(in crate::modules::auth) struct GitHubProfile {
    pub(in crate::modules::auth) github_user_id: GitHubId,
    pub(in crate::modules::auth) login: String,
    pub(in crate::modules::auth) name: Option<String>,
    pub(in crate::modules::auth) email: Option<String>,
    pub(in crate::modules::auth) avatar_url: String,
}
