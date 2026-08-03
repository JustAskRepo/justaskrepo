// shared_kernel/domain_events.rs
//
// The marker trait every domain event implements (ARCHITECTURE.md §6, ADR-004).
// No framework types (rule 9): only std bounds, so events stay decoupled from
// the web framework and async runtime they happen to travel on.

/// Marker trait for everything that travels on the event bus.
///
/// The supertrait bounds are what the bus requires of any event:
/// `Clone` to fan a single event out to many subscribers, `Debug` for tracing,
/// and `Send + Sync + 'static` so it can cross threads and outlive the call that
/// published it.
///
/// `event_name()` is the stable, past-tense wire name used for routing and logs.
/// It is decoupled from the Rust type name so the struct can be renamed without
/// breaking subscribers.
pub trait DomainEvent: Clone + std::fmt::Debug + Send + Sync + 'static {
    fn event_name(&self) -> &'static str;
}
