# JustAskRepo — Claude Instructions

> This file configures Claude's behavior for this codebase.  
> Read `ARCHITECTURE.md` and all files in `ADR/` before making any code changes.

---

## Project Summary

JustAskRepo is a Rust backend (FastAPI being migrated to Rust) + Next.js 15 frontend.  
Backend: Rust, Axum, SQLx (Postgres), Qdrant, OpenAI API, GitHub App.  
Frontend: Next.js 15, NextAuth v5, TypeScript.

The backend follows a **Modular Monolith** architecture. Every architectural decision is documented in `backend/ARCHITECTURE.md` and `backend/ADR/`.

---

## Absolute Rules (Never Violate)

These are non-negotiable. If asked to violate them, refuse and explain why.

### 1. Module Boundaries
- **Never** import from another module's `domain/`, `application/`, or `infrastructure/` layers
- **Only** interact with a module through its `api.rs` public handlers
- **Never** add business logic to `shared_kernel/` — only primitive types, traits, errors, AppContext

```rust
// ❌ WRONG — importing another module's domain type
use crate::modules::indexing::domain::IndexingJob;

// ✅ CORRECT — using the module's public API
use crate::modules::indexing::api::{GetIndexStatusQuery, handle_get_index_status};
```

### 2. Module Structure
Every module MUST have all four layers. Never skip a layer even if it's currently empty:
```
modules/<name>/
├── mod.rs          # re-exports from api.rs only
├── api.rs          # pub Commands, Queries, handler functions
├── domain/         # pure domain logic, no I/O
├── application/    # use cases, orchestration, event handlers
└── infrastructure/ # DB, external API clients
```

### 3. Inter-Module Integration
- **Never** call another module's functions directly from within a module
- State changes between modules happen ONLY via domain events through the EventBus
- Event names must be past tense: `RepoIndexedEvent`, NOT `IndexRepoEvent`
- Events live in `modules/<name>/domain/events.rs`

### 4. CQRS API Pattern
Every module interaction must go through a Command or Query:
```rust
// ❌ WRONG — imperative service method
indexing_service.start_indexing(repo_name).await

// ✅ CORRECT — CQRS command
handle_index_repo(IndexRepoCommand { repo_full_name: repo_name, ... }, ctx).await
```

Naming conventions (strictly enforced):
- Commands: `<Verb><Noun>Command`
- Queries: `Get<Noun>Query` or `List<Noun>Query`  
- Handlers: `handle_<verb>_<noun>` (all async)
- Responses: `<Noun>Response`

### 5. Visibility Rules
```rust
pub           // ONLY api.rs handlers and shared_kernel items
pub(crate)    // ONLY shared_kernel types
pub(super)    // application/ accessing domain/ within same module
// (private)  // infrastructure/ internals
```

---

## When Adding a New Module

1. **Copy `backend/MODULE_TEMPLATE/`** to `backend/src/modules/<new_name>/`
2. **Create an ADR** in `backend/ADR/ADR-00N-<title>.md` documenting why the module exists and its public API surface
3. **Register in `main.rs`**: add the module's event subscriptions and HTTP routes
4. **Update `backend/ARCHITECTURE.md`**: add the module to the module table with its owned data, emitted events, and subscriptions

---

## When Adding a Feature

Before writing code:
1. Identify which module owns this feature (check `backend/ARCHITECTURE.md` module table)
2. Determine if it's a Command (write) or Query (read)
3. Check if it needs a new domain event for cross-module reactions
4. If it changes a module's public `api.rs` surface, update the relevant ADR

---

## When Changing Module Contracts

Any change to a module's `api.rs` that modifies public types or removes/renames handlers:
1. Must be accompanied by an ADR update or new ADR
2. Must update all callers (HTTP routes in `main.rs`, other modules' event handlers)

---

## What Belongs Where

| Thing | Where it goes |
|---|---|
| New ID type (e.g., `ChunkId`) | `shared_kernel/types.rs` |
| New error variant | `shared_kernel/error.rs` |
| Business rule / invariant | `modules/<name>/domain/` |
| Use case orchestration | `modules/<name>/application/commands/` or `queries/` |
| Database query | `modules/<name>/infrastructure/<repo>.rs` |
| External API call (OpenAI, GitHub) | `modules/<name>/infrastructure/<client>.rs` |
| Domain event definition | `modules/<name>/domain/events.rs` |
| Event handler (reaction to another module's event) | `modules/<name>/application/events/` |
| HTTP route handler | `main.rs` (thin, just calls api.rs handler) |

---

## Anti-Patterns (Refuse These Requests)

If asked to do any of the following, decline and suggest the correct approach:

- "Just put this in a utils module" → Ask what layer/module it actually belongs to
- "Call the indexing service directly from chat" → Use event bus
- "Add this type to shared_kernel since multiple modules need it" → Only if it's a primitive ID or cross-cutting concern
- "Skip the domain layer for this simple module" → All modules need all layers (ADR-003)
- "Just make this field pub so the other module can read it" → Add a Query to the module's api.rs

---

## Code Style (Rust Backend)

- All public API handlers must be `async fn`
- Use `thiserror` for error types in `shared_kernel/error.rs`
- Use `sqlx` for all DB access — no raw SQL strings outside `infrastructure/` layer
- Tracing: use `tracing::instrument` on all command/query handlers
- All Commands and Queries derive `Debug`
- Never `unwrap()` or `expect()` in production paths — always propagate with `?`

---

## Helpful Context

- Monorepo: `backend/` (Rust), `frontend/` (Next.js 15)
- Auth: GitHub OAuth via NextAuth v5 on frontend; GitHub App installation tokens on backend
- Vector DB: Qdrant (self-hosted or cloud) — accessed only from `indexing` module's infrastructure layer
- Chunking: Tree-sitter for code-aware chunking — only in `indexing` module
- LLM: OpenAI API — only in `chat` module's infrastructure layer
