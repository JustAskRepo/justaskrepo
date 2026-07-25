# ---- Rust / Axum backend ----
# Multi-stage build with cargo-chef so the dependency layer is cached
# and you don't recompile every crate on each code change.

FROM rust:1-bookworm AS chef
RUN cargo install cargo-chef
# tree-sitter grammars compile C; TLS clients need openssl headers
RUN apt-get update && apt-get install -y build-essential pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
# Binary name comes from [package] name in Cargo.toml
RUN cargo build --release --bin backend

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/backend /usr/local/bin/backend
EXPOSE 8080
# Make sure your server binds to 0.0.0.0:8080, NOT 127.0.0.1 — see README notes
CMD ["backend"]
