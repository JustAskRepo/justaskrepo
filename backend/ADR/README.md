# Architecture Decision Records

This directory contains all Architecture Decision Records (ADRs) for JustAskRepo backend.

ADRs follow the [Michael Nygard format](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions):
- **Context** — the situation that required a decision
- **Decision** — what we decided
- **Rationale** — why we decided this
- **Consequences** — what happens as a result

## Log

| ADR | Title | Status | Date |
|---|---|---|---|
| [ADR-001](ADR-001-modular-monolith.md) | Modular Monolith over Microservices | Accepted | 2026-06-13 |
| [ADR-002](ADR-002-cqrs-module-api.md) | CQRS-Style Module API | Accepted | 2026-06-13 |
| [ADR-003](ADR-003-full-layers-every-module.md) | Full Layered Structure for Every Module | Accepted | 2026-06-13 |
| [ADR-004](ADR-004-event-bus-integration.md) | In-Memory Event Bus as Default Integration | Accepted | 2026-06-13 |
| [ADR-005](ADR-005-architecture-enforcement.md) | Three-Layer Enforcement Strategy | Accepted | 2026-06-13 |
| [ADR-006](ADR-006-shared-kernel-scope.md) | Shared Kernel Scope Restrictions | Accepted | 2026-06-13 |

## How to Add an ADR

1. Copy the template below
2. Number it `ADR-00N` (next in sequence)
3. Name the file `ADR-00N-<short-title>.md`
4. Update this index table
5. Reference the ADR number in the relevant PR

## ADR Template

```markdown
# ADR-00N: <Title>

**Date:** YYYY-MM-DD  
**Status:** Proposed | Accepted | Deprecated | Superseded by ADR-00X  
**Deciders:** <names>

---

## Context

<What situation required this decision?>

## Decision

<What did we decide?>

## Rationale

<Why this option over others?>

## Consequences

- **Positive:** ...
- **Negative:** ...
- **Mitigation:** ...

## Review Trigger

<Under what circumstances should this decision be revisited?>
```
