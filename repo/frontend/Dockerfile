# syntax=docker/dockerfile:1.7
FROM rust:1.88-bookworm AS builder

# Install dioxus-cli from a prebuilt binary via cargo-binstall — avoids
# the multi-minute source compile that `cargo install` would otherwise do.
# cargo-binstall itself is installed from its own prebuilt binary (the
# source build requires rustc >=1.89, which the base image doesn't have).
# curl + ca-certificates are already present in rust:1.88-bookworm.
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
        | bash \
    && cargo binstall -y dioxus-cli@0.6.3

RUN rustup target add wasm32-unknown-unknown

WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
COPY Dioxus.toml ./
COPY src/ src/
COPY assets/ assets/

# BuildKit cache mounts persist the cargo registry and target dir across
# builds. Unique `id=` values prevent collision with the backend build,
# which also mounts /app/target.
RUN --mount=type=cache,target=/usr/local/cargo/registry,id=careops-frontend-cargo-registry \
    --mount=type=cache,target=/usr/local/cargo/git,id=careops-frontend-cargo-git \
    --mount=type=cache,target=/app/target,id=careops-frontend-target \
    dx build --release && \
    cp -r target/dx/careops-frontend/release/web/public /app/public

FROM nginx:alpine

COPY --from=builder /app/public /usr/share/nginx/html
COPY --from=builder /app/assets /usr/share/nginx/html/assets/
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
