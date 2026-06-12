// <module_name>/infrastructure/mod.rs
//
// DB repositories, external API clients (OpenAI, GitHub, Qdrant).
// Visibility: pub(super) only — never exposed outside this module.
//
// Depends on: domain/ (for types), application/ (for repository traits)
// Never imported by: other modules

// pub(super) mod <entity>_repository;
// pub(super) mod <external_service>_client;
