// infrastructure/mod.rs
//
// Cross-cutting machinery: pools, config, the event bus, HTTP wiring. This is
// composition-root code, not domain code — if a module were extracted into its
// own service, this directory would be rewritten rather than carried along.
//
// Not a home for `utils`. Anything that is neither machinery nor a shared
// primitive belongs inside a module.

pub mod config;
pub mod context;
pub mod db;
pub mod http;
pub mod rate_limiter;
pub mod valkey;
pub use context::AppContext;
