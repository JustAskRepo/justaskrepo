# JustAskRepo

> AI-powered code intelligence — chat with any GitHub repository.

JustAskRepo lets you connect a GitHub repository and ask questions about it in plain English. It understands your codebase through semantic search, AST-aware chunking, and an LLM-powered chat interface.

---

## Stack

| Layer              | Technology                  |
| ------------------ | --------------------------- |
| Frontend           | Next.js 16 + TypeScript     |
| Backend            | Rust + Axum                 |
| Parsing            | Tree-sitter (Rust bindings) |
| Vector Store       | Qdrant                      |
| Embeddings         | Gemini Embeddings API       |
| LLM                | Gemini                      |
| Database           | PostgreSQL                  |
| Cache / Queue      | Valkey                      |
| GitHub Integration | GitHub App (OAuth + Webhooks) |

---

## Architecture

The backend is structured as a **modular monolith** — a single crate and single deployable binary, organized into domain-bounded modules with enforced boundaries between them. Each module owns its own domain, application, and infrastructure layers, and is reachable only through its `api.rs` Command/Query surface. Modules never call each other directly; state changes propagate as domain events over an in-memory event bus. This keeps the operational simplicity of one service while preserving clean separation of concerns, and leaves the door open to extracting a module into its own service later.

```
justaskrepo/
├── frontend/                     # Next.js 16 web app
└── backend/                      # Rust + Axum (single `backend` crate)
    ├── ARCHITECTURE.md           # Software Architecture Document
    ├── ADR/                      # Architecture Decision Records (ADR-001…006)
    ├── MODULE_TEMPLATE/          # scaffold to copy when adding a module
    └── src/
        ├── main.rs               # composition root: routes + event subscriptions
        ├── modules/
        │   ├── auth/             # GitHub OAuth, session management
        │   ├── installations/    # GitHub App installations, repo access
        │   ├── indexing/         # cloning, Tree-sitter chunking, embedding, Qdrant
        │   ├── chat/             # RAG pipeline, conversations, LLM calls
        │   └── webhooks/         # webhook ingestion, signature verification, routing
        ├── shared_kernel/        # primitives only: types, errors, DomainEvent trait
        └── infrastructure/       # cross-cutting: db pool, config, event bus
```

Every module has the same four layers — `api.rs`, `domain/`, `application/`, `infrastructure/` — with no exceptions. See `backend/ARCHITECTURE.md` for the module responsibility table and `backend/ADR/` for the reasoning behind each rule.

> Module scaffolding is in progress: `MODULE_TEMPLATE/` is the canonical starting point for each of the modules listed above.

---

## Features

- **Repo Chat** — ask questions about any connected repository in natural language
- **Semantic Search** — hybrid BM25 + vector search over code chunks
- **AST-aware Chunking** — Tree-sitter parses code into meaningful chunks, not arbitrary lines
- **GitHub App Integration** — fine-grained repo access via GitHub App installation
- **Multi-repo Support** — connect and query multiple repositories

---

## How it works

1. **Connect** — install the GitHub App on a repository for fine-grained, per-install access.
2. **Index** — the `indexing` module clones the repo, parses source with Tree-sitter into AST-aware chunks that respect function and module boundaries, and generates embeddings via the Gemini Embeddings API.
3. **Store** — chunks and their vectors land in Qdrant; metadata lives in PostgreSQL. Valkey caches hot data (embeddings, repeated query results, GitHub API responses) and backs an async job queue so repo indexing runs off the request path.
4. **Retrieve** — queries run through a hybrid pipeline combining BM25 keyword matching with vector similarity, returning semantically coherent code units.
5. **Answer** — retrieved context grounds a Gemini-powered chat interface that responds with relevant code.

---

## Status

Actively under development.