// auth/application/events/mod.rs
//
// Handlers for domain events published by OTHER modules.
// This module's reactions to the outside world.
// Subscriptions are registered in api.rs and started from main.rs.

pub(crate) mod on_repo_uninstalled;
