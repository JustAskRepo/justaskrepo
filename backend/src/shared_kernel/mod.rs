// shared_kernel/mod.rs
//
// Types and traits with NO behaviour and NO framework dependencies (ADR-006).
// If a module were extracted into its own service tomorrow, it would take this
// directory with it unchanged.
//
// Capped at three things:
//   1. Primitive ID newtypes      -> types.rs
//   2. The DomainEvent trait      -> domain_events.rs
//   3. AppError                   -> error.rs
//
// AppContext (the DI container) deliberately lives in `infrastructure/context.rs`
// instead: it holds a sqlx pool, and architecture rule 9 forbids sqlx here.

pub mod domain_events;
pub mod error;
pub mod types;
