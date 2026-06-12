# ADR-004: In-Memory Event Bus as Default Integration Style

**Date:** 2026-06-13  
**Status:** Accepted  
**Deciders:** Arya Sharma

---

## Context

Modules need to react to each other's state changes. Kamil Grzybek's article identifies four integration styles (in increasing coupling order): File Transfer → Shared DB → Direct Call → Messaging. Multiple styles can coexist. We needed to pick a default.

Options considered:
1. **Direct synchronous calls** — `chat_module::api::handle_xxx()` called directly from `indexing`
2. **Shared database** — modules read each other's tables
3. **In-memory async event bus** — modules publish events, others subscribe
4. **Mixed** — sync for queries, async for state changes

## Decision

We use **in-memory async event bus as the default integration style** for all cross-module state change reactions. Direct calls between modules are **prohibited**.

## Rationale

- Direct calls create hidden compile-time coupling — module A imports module B's `api.rs`, meaning A can never be extracted without B
- Shared DB creates semantic coupling — module A knows about module B's schema
- Event bus keeps modules genuinely independent: `indexing` doesn't know `chat` exists
- In-memory (tokio broadcast/mpsc channels) keeps it simple — no Kafka, no Redis pub/sub at this stage
- The event bus interface is designed so it can be replaced with a real message broker (NATS, RabbitMQ) by changing only `infrastructure/event_bus.rs`

## Event Design Rules

- Events are named in **past tense**: `RepoIndexedEvent`, `UserAuthenticatedEvent`
- Events contain **only IDs and minimal data** — subscribers fetch full data via queries if needed
- Events are **immutable** — once published, never modified
- Events are defined in `modules/<name>/domain/events.rs`
- The `DomainEvent` marker trait is in `shared_kernel/domain_events.rs`

## What Direct Calls ARE Allowed For

- HTTP route handlers calling a module's `api.rs` command/query handlers (this is by design)
- `webhooks` module routing to other modules' command handlers (it's a dispatcher, not a domain module)

## Consequences

- **Positive:** Modules are genuinely independent — can be extracted without changing business logic
- **Positive:** Event log is auditable (can be persisted to DB as outbox pattern later)
- **Negative:** Async event flow is harder to trace than synchronous call chains
- **Mitigation:** All events are logged at DEBUG level with correlation IDs; architecture tests verify no direct cross-module imports
