// <module_name>/infrastructure/mod.rs
//
// DB repositories, external API clients (Gemini, GitHub, Qdrant).
// Visibility: pub(super) only — never exposed outside this module.
//
// Depends on: domain/ (for types), application/ (for repository traits)
// Never imported by: other modules

// pub(super) mod <entity>_repository;
// pub(super) mod <external_service>_client;
pub(super) mod github_oauth_client;
pub(super) mod oauth_state_store;
pub(super) mod session_store;
pub(super) mod user_repository;
