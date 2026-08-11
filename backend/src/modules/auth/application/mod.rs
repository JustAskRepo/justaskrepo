// <module_name>/application/mod.rs
//
// Use cases: command handlers and query handlers.
// Orchestrates domain logic and infrastructure calls.
// Publishes domain events via ctx.event_bus.
//
// Depends on: domain/, shared_kernel/
// Never depends on: infrastructure/ directly (use traits/ports)

pub(super) mod commands;
pub(super) mod events;
pub(super) mod queries; // event handlers (subscriptions from other modules)
