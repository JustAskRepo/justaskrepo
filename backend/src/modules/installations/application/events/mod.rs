// installations/application/events/mod.rs
//
// Handlers for domain events published by OTHER modules.
//
// Empty, and expected to stay that way: this module is called by the
// composition root rather than reacting to the bus (ADR-009 decision 2). The
// layer exists because ADR-003 makes no layer optional.
