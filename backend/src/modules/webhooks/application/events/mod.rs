// webhooks/application/events/mod.rs
//
// Handlers for domain events published by OTHER modules.
//
// Empty by design, and expected to stay so. This module is upstream of the bus:
// it turns an HTTP request into a command, and everything downstream reacts to
// what that command publishes.
