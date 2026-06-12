# ADR-002: CQRS-Style Module API (Commands & Queries)

**Date:** 2026-06-13  
**Status:** Accepted  
**Deciders:** Arya Sharma

---

## Context

Modules need a well-defined public interface. Options considered:

1. **Traditional service methods** — `IndexingService::start_indexing()`, `ChatService::ask()`
2. **CQRS-style Commands & Queries** — `IndexRepoCommand`, `GetIndexStatusQuery` dispatched via handler functions in `api.rs`
3. **Full mediator pattern** — a runtime dispatcher that resolves handlers by type

## Decision

We adopt **CQRS-style Commands & Queries via explicit handler functions in `api.rs`**. No runtime mediator/dispatcher.

## Rationale

- Commands and Queries as plain structs make intent explicit and self-documenting
- Separation of write (Command) and read (Query) intent prevents service methods that accidentally mix reads and writes
- Explicit handler functions (not a mediator) keep Rust's type system fully in play — no `Any` downcasting, no dynamic dispatch overhead
- The HTTP router calls handlers directly: `handle_index_repo(cmd, ctx).await` — simple, traceable
- Enables future extraction: the same Command struct becomes a gRPC/HTTP request body with no business logic change

## Naming Conventions

- Commands: `<Verb><Noun>Command` — e.g., `IndexRepoCommand`, `SendMessageCommand`
- Queries: `Get<Noun>Query` or `List<Noun>Query` — e.g., `GetIndexStatusQuery`, `ListConversationsQuery`
- Handlers: `handle_<verb>_<noun>` — e.g., `handle_index_repo`, `handle_get_index_status`
- Responses: `<Verb><Noun>Response` or `<Noun>Response` — e.g., `IndexRepoResponse`, `IndexStatusResponse`

## Consequences

- **Positive:** Every module's capabilities are enumerable from `api.rs` alone
- **Positive:** Testing is simple — construct a Command, call the handler, assert the Response
- **Positive:** No magic; the full call chain is visible in IDE navigation
- **Negative:** More boilerplate than direct service methods
- **Mitigation:** The boilerplate is intentional — it is the documentation
