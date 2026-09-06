use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GitHubId(pub i64);

/// An opaque session identifier — a bearer credential. `Debug` is implemented by
/// hand rather than derived so that formatting a Command, Response, or tracing
/// span can never put a live session ID into the logs.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl fmt::Debug for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionId(<redacted>)")
    }
}

/// Ties an event back to the request that caused it.
///
/// A field on every event rather than ambient context: an ID that lives in the
/// event survives the move to a real broker, and one that lives in the tracing
/// context does not. It is the answer to the one downside ADR-004 accepts —
/// that an async reaction is harder to follow than a call chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(pub Uuid);

impl CorrelationId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
