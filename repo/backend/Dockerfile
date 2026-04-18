# syntax=docker/dockerfile:1.7
FROM rust:1.88-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
COPY migrations/ migrations/

# BuildKit cache mounts persist the cargo registry and target dir across
# builds, so dependency recompiles are skipped on subsequent runs.
# Unique `id=` values prevent collision with the frontend build, which also
# mounts /app/target.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=careops-backend-cargo-registry \
    --mount=type=cache,target=/usr/local/cargo/git,id=careops-backend-cargo-git \
    --mount=type=cache,target=/app/target,id=careops-backend-target \
    cargo build --release && \
    cp target/release/careops-backend /app/careops-backend

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/careops-backend .
COPY --from=builder /app/migrations ./migrations

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000

EXPOSE 8000

HEALTHCHECK --interval=2s --timeout=3s --start-period=5s --retries=60 \
    CMD curl -fsS http://localhost:8000/api/health/live || exit 1

CMD ["./careops-backend"]
