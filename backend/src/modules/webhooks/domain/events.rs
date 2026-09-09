// webhooks/domain/events.rs
//
// Deliberately empty. This module publishes nothing: it verifies a request,
// works out what it means, and calls the module that owns the data. The events
// that follow are `installations`' to publish, because they describe changes to
// its state and not to ours (ADR-004).
