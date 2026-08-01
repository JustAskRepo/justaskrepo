# ---- JustAskRepo: single-binary image ----
# Axum serves both the JSON/SSE API and the Next.js static export from one
# process. Build order: static frontend -> Rust binary -> slim runtime that
# carries the binary plus the exported assets.

# ---- Stage 1: Next.js static export (emits frontend/out) ----
FROM node:22-alpine AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci
COPY frontend/ ./
# next.config.ts sets `output: 'export'` -> writes ./out
RUN npm run build

# ---- Stage 2: cargo-chef dependency plan (cache layer) ----
FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef
# tree-sitter grammars compile C; TLS clients need openssl headers
RUN apt-get update && apt-get install -y build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

FROM chef AS planner
COPY backend/ .
RUN cargo chef prepare --recipe-path recipe.json

# ---- Stage 3: build the Axum binary ----
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY backend/ .
# Binary name comes from [package] name in backend/Cargo.toml
RUN cargo build --release --bin justaskrepo

# ---- Stage 4: runtime — one binary + static assets ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/justaskrepo /usr/local/bin/justaskrepo
# Axum serves these via ServeDir(STATIC_DIR) with out/index.html as the
# client-router fallback (.not_found_service).
COPY --from=frontend /app/frontend/out /app/static
ENV STATIC_DIR=/app/static
EXPOSE 8080
# Bind to 0.0.0.0:8080, NOT 127.0.0.1 — see README notes
CMD ["justaskrepo"]
