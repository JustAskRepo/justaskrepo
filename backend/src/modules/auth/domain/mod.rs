// <module_name>/domain/mod.rs
//
// Pure domain logic. No async. No I/O. No framework dependencies.
// Contains: entities, value objects, domain events, domain rules/invariants.
//
// Visibility: types here are pub(super) — visible within this module only.
// Never pub — outsiders use the module via api.rs only.

pub(super) mod events;
pub(super) mod github_profile;
pub(super) mod random_token;
pub(super) mod session;
pub(super) mod user;
