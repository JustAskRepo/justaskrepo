// <module_name>/api.rs
//
// This is the ONLY public surface of this module.
// All Commands, Queries, and their handler functions live here.
// Handlers are thin — they delegate to application/ use cases.

use crate::shared_kernel::{error::AppError, AppContext};

// ─── Commands ────────────────────────────────────────────────────────────────

/// Intent to <describe write operation>.
#[derive(Debug)]
pub struct Example_Verb_NounCommand {
    // pub field: Type,
}

#[derive(Debug)]
pub struct Example_Verb_NounResponse {
    // pub field: Type,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_example_verb_noun(
    cmd: Example_Verb_NounCommand,
    ctx: &AppContext,
) -> Result<Example_Verb_NounResponse, AppError> {
    todo!("Delegate to application/commands/example_verb_noun.rs")
}

// ─── Queries ─────────────────────────────────────────────────────────────────

/// Read-only intent to <describe read operation>. No side effects.
#[derive(Debug)]
pub struct GetExampleQuery {
    // pub field: Type,
}

#[derive(Debug)]
pub struct ExampleResponse {
    // pub field: Type,
}

#[tracing::instrument(skip(ctx))]
pub async fn handle_get_example(
    query: GetExampleQuery,
    ctx: &AppContext,
) -> Result<ExampleResponse, AppError> {
    todo!("Delegate to application/queries/get_example.rs")
}
