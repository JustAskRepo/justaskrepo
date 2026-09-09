// webhooks/domain/mod.rs
//
// Pure domain logic. No async. No I/O. No framework dependencies.
//
// This module is a dispatcher, not a domain (ADR-004), so there is little here
// by design — but signature verification belongs in exactly this layer: it is a
// pure function over bytes, it decides whether a request is real, and it is the
// single highest-value thing in the module to test.

pub(super) mod events;
pub(super) mod signature;
