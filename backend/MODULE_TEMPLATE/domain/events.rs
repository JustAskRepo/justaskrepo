// <module_name>/domain/events.rs
//
// Domain events produced by this module.
// Events are named in PAST TENSE: ThingHappenedEvent
// Events contain only IDs + minimal data needed by subscribers.
// Full data is fetched by subscribers via Queries if needed.

use crate::shared_kernel::{domain_events::DomainEvent, types::*};
use serde::{Deserialize, Serialize};

// Example — replace with actual events:

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleThingHappenedEvent {
    pub id: String, // use a shared_kernel ID type
    pub occurred_at: chrono::DateTime<chrono::Utc>,
}

impl DomainEvent for ExampleThingHappenedEvent {
    fn event_name(&self) -> &'static str {
        "example.thing_happened"
    }
}
