# JustAskRepo

> AI-powered code intelligence — chat with any GitHub repository.

JustAskRepo lets you connect a GitHub repository and ask questions about it in plain English. It understands your codebase through semantic search, AST-aware chunking, and an LLM-powered chat interface.

---

## Stack

| Layer | Technology |
|---|---|
| Frontend | Next.js 15 + NextAuth v5 |
| Backend | Rust + Axum |
| Parsing | Tree-sitter (Rust bindings) |
| Vector Store | Qdrant |
| Embeddings | Gemini Embeddings API |
| LLM | Gemini |
| Database | PostgreSQL |
| GitHub Integration | GitHub App (OAuth + Webhooks) |

---

## Structure

```
justaskrepo/
├── frontend/     # Next.js 15 web app
└── backend/      # Rust + Axum API server
```

---

## Features

- **Repo Chat** — ask questions about any connected repository in natural language
- **Semantic Search** — hybrid BM25 + vector search over code chunks
- **AST-aware Chunking** — Tree-sitter parses code into meaningful chunks, not arbitrary lines
- **GitHub App Integration** — fine-grained repo access via GitHub App installation
- **Multi-repo Support** — connect and query multiple repositories

---

## Status

Actively under development. Hobby project — no deadlines, built for learning and fun.
