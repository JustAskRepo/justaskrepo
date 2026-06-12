# ADR-003: Full Layered Structure for Every Module

**Date:** 2026-06-13  
**Status:** Accepted  
**Deciders:** Arya Sharma

---

## Context

Kamil Grzybek's article explicitly states: *"the application of layers should not be a global decision — apply them where the domain is complex enough."* We considered:

1. **Full layers always** — every module has `domain/`, `application/`, `infrastructure/` regardless of complexity
2. **Layers on demand** — simple modules (like `auth`) skip layers and use vertical slices

## Decision

We use **full layers for every module**, even simple ones.

## Rationale

- JustAskRepo is a Rust project with a small team. Consistency outweighs flexibility at this team size.
- The cost of empty folders is zero. The cost of inconsistent structure is high onboarding friction.
- "Simple" modules tend to grow. `auth` will eventually own token refresh, permission scopes, and session expiry — the layers will be needed.
- Architecture tests are easier to write when the structure is predictable: "every module MUST have a `domain/` folder" is a trivial assertion.
- Rust's module system maps 1:1 to folder structure — the layer structure IS the visibility structure.

## Layer Rules

| Layer | Depends On | Never Depends On |
|---|---|---|
| `domain/` | nothing (pure Rust, no async, no I/O) | `application/`, `infrastructure/`, other modules |
| `application/` | `domain/`, `shared_kernel/` | `infrastructure/` directly |
| `infrastructure/` | `domain/`, `application/` traits | other modules' `infrastructure/` |

## Consequences

- **Positive:** Predictable structure across all modules
- **Positive:** Architecture tests can assert structure uniformly
- **Negative:** Thin modules have "empty" layer folders initially
- **Mitigation:** Each layer folder gets a `mod.rs` with a comment explaining what belongs there
