# ADR-006: Shared Kernel Scope Restrictions

**Date:** 2026-06-13  
**Status:** Accepted  
**Deciders:** Arya Sharma

---

## Context

A shared kernel is needed for primitive types and cross-cutting concerns. Without strict scope rules, it becomes a "utils" dumping ground and creates coupling between all modules.

## Decision

The `shared_kernel/` module is strictly limited to:

1. **Primitive ID types** (newtypes): `UserId`, `InstallationId`, `RepoFullName`, `ConversationId`, `MessageId`
2. **`DomainEvent` trait** — marker trait for all domain events
3. **`AppError`** — unified error enum (variants added per module, not per module's internals)
4. **`AppContext`** — the dependency injection container holding DB pool, event bus, config

## What is Explicitly Prohibited in Shared Kernel

- Business logic of any kind
- DTO types for API responses
- Module-specific domain types (e.g., `IndexingJob`, `Conversation`)
- Database models or ORM types
- Any type that would require one module to know about another's domain

## Enforcement

Once architecture tests are added: `tests/architecture/shared_kernel_tests.rs` will assert that `shared_kernel/` has no imports from any `modules/` path.

## Rationale

- If the shared kernel grows unchecked, all modules become implicitly coupled through it
- Newtypes (`UserId(String)`) are safe because they carry no business semantics — they're just strongly-typed identifiers
- `AppContext` is allowed because it's the composition root, not a domain concept

## Consequences

- **Positive:** Modules remain genuinely independent
- **Positive:** The shared kernel can be audited in minutes — it's intentionally tiny
- **Negative:** Some duplication across modules (e.g., each module defines its own response DTOs)
- **Mitigation:** Duplication in DTOs is intentional — it prevents coupling on implementation details
