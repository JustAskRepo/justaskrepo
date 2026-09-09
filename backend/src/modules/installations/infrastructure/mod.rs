// installations/infrastructure/mod.rs
//
// DB repositories, external API clients (GitHub App JWT, installation tokens).
// Visibility: pub(super) only — never exposed outside this module.
//
// Depends on: domain/ (for types), application/ (for repository traits)
// Never imported by: other modules

pub(super) mod github_api;
pub(super) mod github_app_client;
pub(super) mod github_user_client;
pub(super) mod installation_repository;
pub(super) mod installation_sync_throttle;
pub(super) mod installation_token_cache;
