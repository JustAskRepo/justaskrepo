// infrastructure/http/routes/mod.rs
//
// One file per module. Each is a thin adapter with exactly four jobs: extract,
// build the Command/Query, call the module's api.rs handler, map the result to
// a response. No business logic, no DB access, no branching on domain state.
//
// A route handler MAY call several modules — that is the composition root's
// privilege and the escape hatch for cross-module flows (ARCHITECTURE.md §5.1).
// A module may not.

pub mod health;

// TODO: auth.rs, repositories.rs, chat.rs, webhooks.rs — as each module lands.
