// webhooks/application/mod.rs
//
// Use cases: command handlers and query handlers.
//
// Depends on: domain/, shared_kernel/
// Never depends on: infrastructure/ directly (use traits/ports)

pub(super) mod commands;
pub(super) mod events;
pub(super) mod queries;
