# JustAskRepo — Architecture Documentation

> **Last Updated:** 2026-06-13  
> **Status:** Active  
> **Architect:** Arya Sharma

---

## 1. Overview

JustAskRepo is an AI-powered tool that lets users chat with any GitHub repository using Retrieval-Augmented Generation (RAG). Users authenticate via GitHub OAuth, install a GitHub App on a repository, and then query that repository's code through a conversational interface.

This document is the **Software Architecture Document (SAD)** for the Rust backend. It captures architectural decisions, module boundaries, integration styles, and enforcement mechanisms. Every decision here has a corresponding ADR in `ADR/`.

---

## 2. Architecture Style: Modular Monolith

### Why Modular Monolith?

JustAskRepo is in an early, exploratory phase. The domain boundaries are not fully understood yet. A modular monolith gives us:

- **Simplicity of deployment** — single binary, no distributed systems overhead
- **Refactorability** — modules with hard boundaries can be extracted to microservices later without a big-bang rewrite
- **Discoverability** — all code in one place, easy to navigate and onboard
- **Strong consistency** — no network partitions, transactions work across use cases if needed

> See ADR-001 for the full decision record.

### What "Modular" Means Here

A module is **not** just a folder. It satisfies all three of these:
1. **Independent** — can be developed, tested, and reasoned about in isolation
2. **Self-contained** — owns its own data access, its own domain logic, its own composition
3. **Well-defined interface** — the ONLY way to interact with a module is through its `api.rs` public surface, via Commands and Queries

Modules do **not** share:
- Internal domain types
- Database tables (except through explicit public APIs)
- Application service logic

---

## 3. System Modules

```
backend/src/
├── modules/
│   ├── auth/           # GitHub OAuth, session management
│   ├── installations/  # GitHub App installations, repo access
│   ├── indexing/       # Repo cloning, Tree-sitter chunking, embedding, Qdrant storage
│   ├── chat/           # RAG pipeline, conversation management, OpenAI calls
│   └── webhooks/       # GitHub App webhook ingestion, event routing
├── shared_kernel/      # Primitives shared across all modules (no business logic)
│   ├── domain_events.rs
│   ├── error.rs
│   └── types.rs
├── infrastructure/     # Cross-cutting infra (DB pool, HTTP client, config)
│   ├── db.rs
│   ├── config.rs
│   └── event_bus.rs
└── main.rs
```

### Module Responsibilities

| Module | Owns | Emits Events | Listens To |
|---|---|---|---|
| `auth` | User identity, sessions, GitHub tokens | `UserAuthenticatedEvent` | — |
| `installations` | GitHub App installations, repo allowlist | `RepoInstalledEvent`, `RepoUninstalledEvent` | `UserAuthenticatedEvent` |
| `indexing` | Clone queue, chunks, embeddings, Qdrant collections | `RepoIndexedEvent`, `IndexingFailedEvent` | `RepoInstalledEvent` |
| `chat` | Conversations, messages, RAG context assembly | `ConversationCreatedEvent` | `RepoIndexedEvent` |
| `webhooks` | Raw webhook parsing, signature verification | — | — (routes to other modules) |

---

## 4. Module Internal Structure

Every module **must** follow this exact layer structure. There are no exceptions — even thin modules like `auth` use all layers (see ADR-003).

```
modules/<name>/
├── mod.rs              # Public re-exports ONLY. This is the module boundary.
├── api.rs              # Command/Query handlers — the ONLY public API surface
├── domain/
│   ├── mod.rs
│   ├── <entity>.rs     # Domain entities and value objects
│   └── events.rs       # Domain events produced by this module
├── application/
│   ├── mod.rs
│   ├── commands/       # One file per command: <verb>_<noun>.rs
│   ├── queries/        # One file per query: get_<noun>.rs
│   └── events/         # Domain event handlers (subscriptions)
└── infrastructure/
    ├── mod.rs
    └── <repo>.rs       # DB access, external API clients (Qdrant, GitHub, OpenAI)
```

**Visibility rules (enforced by Rust compiler):**
- `domain/` types → `pub(crate)` within the module, never `pub` to the outside
- `application/` → `pub(super)` or `pub(crate)`, never `pub`
- `infrastructure/` → `pub(super)` only, never `pub` or `pub(crate)` to other modules
- `api.rs` handlers → `pub` — this is the ONLY public surface
- `mod.rs` re-exports only what `api.rs` declares

---

## 5. Module API: CQRS-Style Commands & Queries

All inter-module and HTTP-layer communication goes through **Commands** and **Queries**. There are no direct service-to-service calls.

### Commands (write intent)
```rust
// In modules/indexing/api.rs
pub struct IndexRepoCommand {
    pub installation_id: InstallationId,
    pub repo_full_name: String,
    pub requester_user_id: UserId,
}

pub async fn handle_index_repo(
    cmd: IndexRepoCommand,
    ctx: &AppContext,
) -> Result<IndexRepoResponse, AppError> { ... }
```

### Queries (read intent, no side effects)
```rust
// In modules/indexing/api.rs
pub struct GetIndexStatusQuery {
    pub repo_full_name: String,
}

pub async fn handle_get_index_status(
    query: GetIndexStatusQuery,
    ctx: &AppContext,
) -> Result<IndexStatusResponse, AppError> { ... }
```

### Rules
- Commands and Queries are plain structs — no framework magic
- Commands may produce domain events; Queries never do
- The HTTP router in `main.rs` calls these handlers directly
- Modules never call each other's internal functions — only their `api.rs` handlers

> See ADR-002 for the full decision record.

---

## 6. Integration Style: In-Memory Event Bus (Messaging Dominant)

Modules communicate **asynchronously via domain events** for all state changes. This is the default integration style. Direct calls are **not used** between modules.

### Event Bus Design
```rust
// infrastructure/event_bus.rs
pub struct EventBus {
    // tokio broadcast channel per event type
}

impl EventBus {
    pub async fn publish<E: DomainEvent>(&self, event: E) -> Result<()>
    pub fn subscribe<E: DomainEvent>(&self) -> EventReceiver<E>
}
```

### Event Flow Example
```
[HTTP Request: POST /repos/:name/index]
        │
        ▼
[IndexRepoCommand handler] — application logic
        │
        ▼  publishes
[RepoQueuedForIndexingEvent] → EventBus
        │
        ├──▶ [IndexingModule: clones + chunks + embeds]
        │           │ publishes
        │           ▼
        │    [RepoIndexedEvent] → EventBus
        │           │
        │           └──▶ [ChatModule: creates default conversation]
        │
        └──▶ [WebhooksModule: logs the indexing job start]
```

### Rules
- State changes ALWAYS emit an event, never call another module directly
- Event handlers are registered at startup in `main.rs`
- Events are plain Rust structs deriving `Clone + Debug + Serialize`
- Events live in `modules/<name>/domain/events.rs`
- The `shared_kernel/domain_events.rs` defines the `DomainEvent` trait

> See ADR-004 for the full decision record.

---

## 7. Shared Kernel

The shared kernel is **not a utilities dumping ground**. It contains only:

1. **Primitive types** — `UserId`, `InstallationId`, `RepoFullName`, `ConversationId` (newtype wrappers)
2. **The `DomainEvent` trait** — marker trait for all domain events
3. **`AppError`** — unified error enum for all modules
4. **`AppContext`** — the dependency container passed to all handlers

```rust
// shared_kernel/types.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(pub String);

#[derive(Debug, Clone)]
pub struct RepoFullName(pub String);
```

**What does NOT go in shared kernel:**
- Business logic of any kind
- Module-specific types or DTOs
- Anything that causes one module to depend on another's internals

---

## 8. Architecture Enforcement

Enforcement happens at three layers (defense in depth):

### Layer 1: Rust Compiler (Strongest)
- `pub(super)` — visible only within the module's own files
- `pub(crate)` — visible across the crate but only for shared kernel types
- `pub` — only on `api.rs` command/query handlers and shared kernel items

### Layer 2: Automated Architecture Tests (`tests/architecture/`)
Run with `cargo test --test architecture`. Fails CI if violated. Tests include:
- No module imports another module's non-api types
- No `infrastructure/` types leak into `domain/`
- All domain events implement `DomainEvent` trait
- All public functions in `api.rs` are `async`

> Tests will be added once the initial module scaffolding is in place.

### Layer 3: Code Review + ADRs
- Every PR touching module boundaries requires review against this document
- Any change to module contracts requires a new or updated ADR
- Architecture violations are `P0` — block merge
- PR template (`.github/PULL_REQUEST_TEMPLATE.md`) includes the full checklist

> See ADR-005 for the enforcement strategy decision record.

---

## 9. Architecture Decision Log

| ADR | Title | Status |
|---|---|---|
| [ADR-001](ADR/ADR-001-modular-monolith.md) | Modular Monolith over Microservices | Accepted |
| [ADR-002](ADR/ADR-002-cqrs-module-api.md) | CQRS-Style Module API (Commands & Queries) | Accepted |
| [ADR-003](ADR/ADR-003-full-layers-every-module.md) | Full Layered Structure for Every Module | Accepted |
| [ADR-004](ADR/ADR-004-event-bus-integration.md) | In-Memory Event Bus as Default Integration Style | Accepted |
| [ADR-005](ADR/ADR-005-architecture-enforcement.md) | Three-Layer Architecture Enforcement Strategy | Accepted |
| [ADR-006](ADR/ADR-006-shared-kernel-scope.md) | Shared Kernel Scope Restrictions | Accepted |

---

## 10. Future: Path to Microservices

The modular monolith is designed to be extracted. When a module needs independent scaling:
1. Its `api.rs` becomes the service contract (gRPC or HTTP)
2. Its event bus subscriptions become message queue subscriptions (NATS / Kafka)
3. Its `infrastructure/` DB layer gets its own database
4. No business logic changes required — only transport layer changes

The modules that are most likely candidates for extraction first: `indexing` (CPU/IO heavy), `chat` (latency-sensitive).
