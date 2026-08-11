# ADR-005: Three-Layer Architecture Enforcement Strategy

**Date:** 2026-06-13  
**Status:** Accepted  
**Deciders:** Arya Sharma

---

## Context

Architecture decisions mean nothing without enforcement. Kamil Grzybek's article identifies three enforcement mechanisms: compiler, automated tests, code review. We adopt all three (defense in depth).

## Decision

We enforce architecture at three layers, each catching different violation types.

## Layer 1: Rust Compiler (Immediate, Strongest)

Rust's visibility modifiers are our first line of defense:

```
pub           → only api.rs handlers and shared_kernel items
pub(crate)    → shared_kernel types only  
pub(super)    → application/ accessing domain/ within same module
(default)     → private, infrastructure/ internals
```

Any cross-module internal import is a **compile error**. This is free and instant.

## Layer 2: Automated Architecture Tests (`tests/architecture/`)

Run via `cargo test --test architecture`. These tests assert structural rules the compiler cannot:

- Module `domain/` has no dependency on `infrastructure/` (via import analysis)
- All types in `api.rs` are public and async
- No module's `mod.rs` re-exports anything other than `api.rs` items
- All domain events implement the `DomainEvent` trait
- Event names end in `Event` suffix

These tests run in CI and **block merge** on failure.

> Architecture tests will be added once initial module scaffolding is complete.

## Layer 3: Code Review + ADR Process

- A PR that adds a new module requires a new ADR **only if the module embodies a non-obvious decision** (an auth model, storage choice, or boundary with a real alternative). A self-justifying module is registered in `ARCHITECTURE.md`'s module table and documented in its `api.rs` doc comments — no ADR. Documentation existence is a judgment call and is deliberately a review gate, not an automated test (see Layer 2, which asserts *structure* only).
- Any PR that changes a module's `api.rs` public surface updates `ARCHITECTURE.md`'s module table, and updates an ADR only where an ADR made a claim about that surface
- PRs are reviewed against `ARCHITECTURE.md` explicitly
- PR template includes architecture checklist (`.github/PULL_REQUEST_TEMPLATE.md`)

## Architecture Violation Severity

| Violation | Severity | Action |
|---|---|---|
| Cross-module internal import | P0 | Block merge immediately |
| Missing layer folder | P1 | Block merge |
| Domain event not implementing trait | P1 | Block merge |
| New module registered but not in ARCHITECTURE.md module table | P2 | Require before merge |
| Missing ADR for a module that embodies a non-obvious decision | P2 | Require before merge |
| api.rs handler not async | P2 | Require before merge |

## Consequences

- **Positive:** Violations are caught at the earliest possible stage
- **Positive:** New contributors get immediate compiler feedback on boundary violations
- **Negative:** Architecture tests require maintenance as structure evolves
- **Mitigation:** Architecture tests are simple string/import checks — low maintenance cost
