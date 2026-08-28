use crate::shared_kernel::types::{GitHubId, UserId};

#[derive(Debug)]
pub(in crate::modules::auth) struct User {
    pub(in crate::modules::auth) id: UserId,
    pub(in crate::modules::auth) github_id: GitHubId,
    pub(in crate::modules::auth) username: String,
    pub(in crate::modules::auth) name: Option<String>,
    pub(in crate::modules::auth) email: Option<String>,
    pub(in crate::modules::auth) avatar_url: Option<String>,
}
