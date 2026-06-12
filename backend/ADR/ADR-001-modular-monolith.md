# ADR-001: Modular Monolith over Microservices

**Date:** 2026-06-13  
**Status:** Accepted  
**Deciders:** Arya Sharma

---

## Context

JustAskRepo is a new product. Domain boundaries between indexing, chat, auth, and webhook processing are not yet fully stable. We need to ship fast, iterate, and learn.

Two primary options were considered:
- **Microservices** — independent deployable services per domain
- **Modular Monolith** — single deployable unit with enforced module boundaries

## Decision

We adopt a **Modular Monolith** architecture.

## Rationale

- Domain boundaries are still being discovered; premature service extraction would lead to wrong cuts and expensive refactoring across network boundaries
- A single Rust binary is vastly simpler to deploy, debug, and test at this stage
- Rust's ownership model and module visibility rules give us near-compile-time enforcement of boundaries without needing a service mesh
- The modules are designed to be extractable: `api.rs` becomes the service contract, event bus subscriptions become queue subscriptions, infra layer gets its own DB — all without changing business logic

## Consequences

- **Positive:** Fast iteration, simple deployment, strong consistency, low operational overhead
- **Positive:** Module boundaries are enforced at compile time via Rust visibility
- **Negative:** All modules scale together; can't independently scale `indexing` until extraction
- **Mitigation:** `indexing` is designed as async/queue-based from day one so it can be extracted with minimal disruption

## Review Trigger

Revisit this decision when any single module requires independent deployment or when the team grows beyond 4 engineers working on the same codebase simultaneously.
