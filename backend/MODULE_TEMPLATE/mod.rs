// <module_name>/mod.rs
//
// This file is the module boundary.
// It ONLY re-exports items declared in api.rs.
// Never re-export from domain/, application/, or infrastructure/.

pub use self::api::*;

mod api;
mod domain;
mod application;
mod infrastructure;
