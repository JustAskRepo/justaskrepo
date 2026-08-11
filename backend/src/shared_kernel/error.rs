// shared_kernel/error.rs
//
// The one error type every module returns. Framework-free by design: nothing in
// here knows what an HTTP status code is. The mapping to a response lives in
// `infrastructure/http/error.rs`, the only file allowed to know both.
//
// Rust's orphan rule permits that split — AppError is local to the crate, so a
// foreign trait like axum's IntoResponse can be implemented for it from anywhere
// in the crate (ARCHITECTURE.md §7).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    /// The thing you asked for does not exist.
    #[error("{resource} not found")]
    NotFound { resource: &'static str },

    /// No valid session — the caller is anonymous.
    #[error("unauthorized")]
    Unauthorized,

    /// Authenticated, but not allowed to do this.
    #[error("forbidden")]
    Forbidden,

    /// The request was understood but is invalid.
    #[error("validation failed: {0}")]
    Validation(String),

    /// The request conflicts with current state (duplicate, concurrent edit).
    #[error("conflict: {0}")]
    Conflict(String),

    /// A dependency is down. The caller may retry.
    #[error("{service} unavailable")]
    Unavailable { service: &'static str },

    /// Anything else. The message is logged, never returned to the client.
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    /// Wrap any error as `Internal` without an `anyhow` import at the call site.
    pub fn internal(err: impl Into<anyhow::Error>) -> Self {
        Self::Internal(err.into())
    }

    /// Stable machine-readable code. Clients branch on this, not on the prose.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "not_found",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::Validation(_) => "validation_failed",
            Self::Conflict(_) => "conflict",
            Self::Unavailable { .. } => "unavailable",
            Self::Internal(_) => "internal_error",
        }
    }
}
