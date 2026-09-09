// Raised from the default 128 because the webhook route's handler future nests
// deeply enough to overflow the trait solver: `post(github)` -> the command ->
// two modules' api.rs -> their use cases -> sqlx and reqwest futures. The error
// it produces names an unrelated tokio type and points at the route, so it is
// worth saying plainly here that this is a type-checking depth limit and not a
// runtime stack limit.
#![recursion_limit = "256"]

pub mod infrastructure;
pub mod modules;
pub mod shared_kernel;
