# JustAskRepo — Architecture Documentation

> **Last Updated:** 2026-08-01  
> **Status:** Active  
> **Architect:** Arya Sharma

---

## 1. Overview

JustAskRepo is an AI-powered tool that lets users chat with any GitHub repository using Retrieval-Augmented Generation (RAG). Users authenticate via GitHub OAuth, install a GitHub App on a repository, and then query that repository's code through a conversational interface.

This document is the **Software Architecture Document (SAD)** for the Rust backend. It captures architectural decisions, module boundaries, integration styles, and enforcement mechanisms. Every decision here has a corresponding ADR in `ADR/`.

### 1.1 Deployment Shape

The system ships as **one binary serving one origin**. This is architecturally significant, so it belongs here rather than only in the deployment config:

- The frontend is a **Next.js static export** (`output: "export"`). There is no Next.js server in production and no BFF layer.
- **Axum serves both**: the exported UI at `/` via `ServeDir`, and the API under `/api/*`.
- In development, the Next dev server on `:3001` rewrites `/api/*` to Axum, reproducing the single origin. Dev is accessed at `:3001`, never `:8080` directly.

**Consequences for the backend:**

| Because | The backend |
| --- | --- |
| UI and API share an origin | needs **no CORS layer** — adding one would only widen surface area |
| The browser calls Axum directly | is the **only** place auth decisions happen; there is no server-side layer in front of it |
| `/` serves static files | must mount every route under `/api` — an unprefixed route is unreachable through the dev proxy |
| Axum 0.8 dropped implicit trailing-slash matching | pairs with `skipTrailingSlashRedirect` in `next.config.ts` so dev and prod see identical paths |

See `AUTHENTICATION.md` §Same-Origin Model for the security consequences.

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

```text
backend/
├── src/
│   ├── modules/
│   │   ├── auth/           # GitHub OAuth, session management
│   │   ├── installations/  # GitHub App installations, repo access
│   │   ├── indexing/       # Repo cloning, Tree-sitter chunking, embedding, Qdrant storage
│   │   ├── chat/           # RAG pipeline, conversation management, Gemini calls
│   │   └── webhooks/       # GitHub App webhook ingestion, event routing
│   ├── shared_kernel/      # Primitives shared across all modules (no business logic)
│   │   ├── mod.rs          # AppContext (the DI container)
│   │   ├── domain_events.rs
│   │   ├── error.rs        # AppError — framework-free, no axum types
│   │   └── types.rs
│   ├── infrastructure/     # Cross-cutting infra — machinery, not domain
│   │   ├── config.rs       # AppConfig — the ONLY file that reads env vars
│   │   ├── db.rs           # Postgres pool
│   │   ├── valkey.rs       # Valkey pool
│   │   ├── event_bus.rs
│   │   └── http/           # HTTP composition layer (see §5.1)
│   │       ├── mod.rs      # router(ctx) -> Router
│   │       ├── error.rs    # impl IntoResponse for AppError
│   │       ├── middleware/ # auth middleware, tracing, rate limiting
│   │       └── routes/     # one file per module: thin adapters to api.rs
│   └── main.rs             # composition root: config → context → subscriptions → serve
├── migrations/             # sqlx migrations
└── tests/
    └── architecture.rs     # boundary rules the compiler can't express (§8)
```

**`shared_kernel/` vs `infrastructure/`** — the distinction that keeps both from becoming a dumping ground:

- `shared_kernel/` holds **types and traits with no behaviour and no framework dependencies**. Its contents are capped by ADR-006 at four things. If a module were pulled out into its own service tomorrow, it would take `shared_kernel` with it unchanged.
- `infrastructure/` holds **shared machinery** — pools, config, the event bus, HTTP wiring. It is composition-root code, not domain code, and would be *rewritten* rather than carried along in an extraction.

Neither is a home for `utils`. Anything that fits neither belongs inside a module.

### Module Responsibilities

| Module | Owns | Emits Events | Listens To |
| --- | --- | --- | --- |
| `auth` | User identity, sessions, GitHub tokens | `UserAuthenticatedEvent`, `UserSessionsRevokedEvent` | `RepoUninstalledEvent` (revoke sessions on uninstall) |
| `installations` | GitHub App installations, repo allowlist | `RepoInstalledEvent`, `RepoUninstalledEvent` | `UserAuthenticatedEvent` |
| `indexing` | Clone queue, chunks, embeddings, Qdrant collections | `RepoIndexedEvent`, `IndexingFailedEvent` | `RepoInstalledEvent` |
| `chat` | Conversations, messages, RAG context assembly | `ConversationCreatedEvent` | `RepoIndexedEvent` |
| `webhooks` | Raw webhook parsing, signature verification | — | — (routes to other modules) |

---

## 4. Module Internal Structure

Every module **must** follow this exact layer structure. There are no exceptions — even thin modules like `auth` use all layers (see ADR-003).

```text
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
    └── <repo>.rs       # DB access, external API clients (Qdrant, GitHub, Gemini)
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
- The HTTP layer calls these handlers directly
- Modules never call each other's internal functions — only their `api.rs` handlers

> See ADR-002 for the full decision record.

### 5.1 The HTTP Layer and the Composition Root

HTTP routes are **not** part of any module. They live in `infrastructure/http/`, which is composition-root code: it knows about every module's `api.rs` and about axum, while no module knows about either.

A route handler is a thin adapter with exactly four jobs:

```rust
// infrastructure/http/routes/auth.rs
pub async fn logout(
    State(ctx): State<AppContext>,
    jar: CookieJar,
    user: CurrentUser,                       // 1. extract (middleware validated it)
) -> Result<(CookieJar, StatusCode), AppError> {
    handle_revoke_session(                   // 2. build the Command, 3. call api.rs
        RevokeSessionCommand {
            session_id: user.session_id,
            user_id: user.user_id,
        },
        &ctx,
    )
    .await?;

    Ok((jar.add(removal_cookie(&ctx.auth)), StatusCode::NO_CONTENT))  // 4. respond
}
```

No business logic, no DB access, no branching on domain state.

**The composition root may call several modules; a module may not.** This is the escape hatch for flows that span modules without violating boundaries. The login callback is the canonical example:

```rust
// ✅ CORRECT — the route handler orchestrates two modules
let session = handle_complete_github_login(cmd, &ctx).await?;
let install = handle_get_installation_status(query, &ctx).await?;   // different module
let dest = if install.has_any { "/dashboard/" } else { INSTALL_URL };

// ❌ WRONG — auth reaching into installations
// (inside modules/auth/application/) let install = installations::...
```

**Middleware follows the same rule.** The auth middleware lives in `infrastructure/http/middleware/`, not in `modules/auth/`. It calls `auth::api::handle_validate_session` like any other caller, which keeps axum types out of the `auth` module entirely.

`main.rs` stays thin: load config → build `AppContext` → register event subscriptions → `serve(http::router(ctx))`.

### 5.2 Middleware

There are two middleware layers today, `require_session` (below) and `rate_limit` (§5.3). `require_session` does two things before a protected route runs — reject cross-origin writes, then turn the session cookie into a `CurrentUser` request extension — and it lives in `infrastructure/http/middleware/` for the same reason routes do: both are HTTP concerns, and the module they lean on must not learn what axum is. It calls `auth::api::handle_get_session` like any other caller.

**Origin verification on mutating methods.** `SameSite=Lax` withholds the session cookie from cross-site sub-resource requests, but still sends it on a top-level cross-site *navigation* — which is always a GET. The unsafe methods are the remaining gap, and the `Origin` header closes it: a browser sets that header itself on every unsafe request and page script cannot forge it, so an origin that is ours is proof the request came from our own page.

```rust
// infrastructure/http/middleware/require_session.rs
verify_origin(request.method(), request.headers(), &ctx.public_origin)?;
```

On `POST`/`PUT`/`PATCH`/`DELETE` the header must be present and equal to the origin derived from `PUBLIC_URL`, or the request is `403` — checked before the Valkey lookup, so a forged request costs nothing. A missing `Origin` on an unsafe method is a rejection, not a pass; that denies a non-browser client the ability to mutate with a session cookie alone, which is the intent.

**The check sits inside the session layer, not on the router.** That placement is the decision, not an implementation detail:

- Cookie authentication is what makes CSRF possible in the first place. A route that authenticates some other way has nothing to defend here.
- `POST /api/webhooks/github` is public, cross-origin by nature, carries no cookie, and is HMAC-verified. An `Origin` check on it would only break it.

The expected origin is **derived** from `PUBLIC_URL` in `config.rs`, never configured on its own. A second variable holding the same fact is a second thing to get out of sync, and out of sync here fails silently in both directions — locking out the real UI, or admitting a stranger.

### 5.3 Rate Limiting

A budget is not an HTTP concern; turning a refusal into a `429` is. So the limiter is split the same way §5.2 splits everything else:

- `infrastructure/rate_limiter.rs` — a fixed-window counter in Valkey, beside `db.rs` and `valkey.rs`. `INCR`, `EXPIRE … NX`, `TTL`, pipelined in one `MULTI`.
- `infrastructure/http/middleware/rate_limit.rs` — resolves the caller, spends one unit, returns `AppError::RateLimited`. It knows no status codes; `http/error.rs` maps that variant to `429` and attaches `Retry-After`.

It is deliberately **not** a module. A module owns data and enforces rules about it; a counter is neither, nothing queries it, and putting it inside `auth` would strand it there when chat and indexing want the same thing (ADR-007).

```rust
// infrastructure/http/mod.rs — one line per limited route group
let auth = routes::auth::routes().route_layer(from_fn_with_state(
    middleware::RateLimitState::new(ctx.clone(), ctx.rate_limits.auth),
    middleware::rate_limit,
));
```

The policy travels *inside* the layer's state rather than being read from `AppContext` at request time, which is what lets the same middleware be mounted more than once with a different budget each time. Adding a budget is a field on `RateLimits` and a `route_layer` line — no second middleware.

**It fails open.** `rate_limiter::check` returns a `RateLimitDecision`, never a `Result`: it is structurally incapable of denying service because of its own failure. A limiter that takes the app down with Valkey has become the outage it exists to prevent, and the exposure is nil regardless — sessions live in Valkey too, so a Valkey outage already means nobody can log in.

**The subject is a trusted-proxy-aware address** (`infrastructure/http/client_ip.rs`). `X-Forwarded-For` is client-writable, so keying on it naively is not a weak limiter but no limiter at all — the caller rotates the header and never spends a budget. `TRUSTED_PROXY_HOPS` says how many proxies really sit in front, and the resolver steps that many entries back from the socket peer; anything further left is an unverifiable claim. It defaults to `0` (ignore the header) because over-trusting is the only genuinely unsafe direction. The login callback records the same value as its audit `ip`.

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
```text
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
- **Framework types** — no `axum`, no `sqlx`, no `reqwest` in `shared_kernel`

### The `AppError` split

Every module returns `AppError`, but only the HTTP layer knows what an HTTP status code is. The two halves live apart:

```rust
// shared_kernel/error.rs — framework-free, thiserror
pub enum AppError { NotFound { .. }, Unauthorized, Forbidden, Validation(..), Conflict(..), RateLimited { .. }, Unavailable { .. }, Internal(..) }

// infrastructure/http/error.rs — the ONE place that maps errors to HTTP
impl IntoResponse for AppError { .. }
```

Rust's orphan rule permits this: `AppError` is local to the crate, so a foreign trait like `IntoResponse` can be implemented for it from anywhere in the crate. The result is one error type across all modules, one status-code mapping, and a `shared_kernel` that survives the extraction path in §10 unchanged.

**Duplication across modules is intended, not a failure.** Two modules needing similar-looking response DTOs is not a reason to hoist a shared type — see ADR-006 §Consequences.

---

## 8. Architecture Enforcement

Enforcement happens at three layers (defense in depth):

### Layer 1: Rust Compiler (Strongest)
- `pub(super)` — visible only within the module's own files
- `pub(crate)` — visible across the crate but only for shared kernel types
- `pub` — only on `api.rs` command/query handlers and shared kernel items

### Layer 2: Automated Architecture Tests (`tests/architecture.rs`)

Run with `cargo test --test architecture`. Fails CI if violated. These are **source-text tests** — they walk `src/` and assert on what the files contain. They need no compilation of the code under test and no test fixtures, which is why they can exist before the modules do.

| # | Rule | Detection |
| --- | --- | --- |
| 1 | No module imports another module's non-`api` path | `use crate::modules::X::{domain,application,infrastructure}` outside module `X` |
| 2 | `domain/` does not depend on `infrastructure/` | `use ...infrastructure` inside any `domain/` file |
| 3 | `domain/` is pure — no I/O, no async | `async fn`, `sqlx`, `reqwest` in `domain/` |
| 4 | Every `pub fn` in `api.rs` is `async` | `pub fn` not preceded by `async` |
| 5 | `mod.rs` re-exports only `api` | any `pub use self::{domain,application,infrastructure}` |
| 6 | Every module has all four layers | directory existence check (ADR-003) |
| 7 | Event names are past tense and end in `Event` | struct names in `domain/events.rs` |
| 8 | `shared_kernel/` imports nothing from `modules/` | `use crate::modules` in `shared_kernel/` |
| 9 | `shared_kernel/` imports no framework | `axum`, `sqlx`, `reqwest` in `shared_kernel/` |
| 10 | `std::env::var` appears only in `config.rs` | text search across `src/` |

`unwrap()` / `expect()` are **not** in this list — they are denied by `[lints.clippy]` in `Cargo.toml`, which is stricter (it understands `#[cfg(test)]`) and reports at the exact span.

> **Status: active from the first module.** These tests are written *before* the auth module lands, so they never have to be retrofitted against existing violations. A rule added after the code it governs is a rule with an exception list.

### Layer 3: Code Review + ADRs
- Every PR touching module boundaries requires review against this document
- Any change to module contracts requires a new or updated ADR
- Architecture violations are `P0` — block merge
- PR template (`.github/PULL_REQUEST_TEMPLATE.md`) includes the full checklist

> See ADR-005 for the enforcement strategy decision record.

---

## 9. Architecture Decision Log

| ADR | Title | Status |
| --- | --- | --- |
| [ADR-001](ADR/ADR-001-modular-monolith.md) | Modular Monolith over Microservices | Accepted |
| [ADR-002](ADR/ADR-002-cqrs-module-api.md) | CQRS-Style Module API (Commands & Queries) | Accepted |
| [ADR-003](ADR/ADR-003-full-layers-every-module.md) | Full Layered Structure for Every Module | Accepted |
| [ADR-004](ADR/ADR-004-event-bus-integration.md) | In-Memory Event Bus as Default Integration Style | Accepted |
| [ADR-005](ADR/ADR-005-architecture-enforcement.md) | Three-Layer Architecture Enforcement Strategy | Accepted |
| [ADR-006](ADR/ADR-006-shared-kernel-scope.md) | Shared Kernel Scope Restrictions | Accepted |
| [ADR-007](ADR/ADR-007-rate-limiting.md) | Rate Limiting as an HTTP Layer over a Valkey Counter | Accepted |

---

## 10. Future: Path to Microservices

The modular monolith is designed to be extracted. When a module needs independent scaling:
1. Its `api.rs` becomes the service contract (gRPC or HTTP)
2. Its event bus subscriptions become message queue subscriptions (NATS / Kafka)
3. Its `infrastructure/` DB layer gets its own database
4. No business logic changes required — only transport layer changes

The modules that are most likely candidates for extraction first: `indexing` (CPU/IO heavy), `chat` (latency-sensitive).
