// webhooks/infrastructure/mod.rs
//
// The wire shapes GitHub sends. Visibility: pub(super) only.
//
// These live in infrastructure rather than domain because they are an external
// system's format, not our model of anything — and because they are read once,
// for the ids in them, and then discarded (ADR-004).

pub(super) mod github_payloads;
