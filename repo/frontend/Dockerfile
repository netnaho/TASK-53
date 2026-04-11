FROM rust:1.88-bookworm AS builder

RUN rustup target add wasm32-unknown-unknown
RUN cargo install dioxus-cli@0.6.3 --locked

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY Dioxus.toml ./
COPY src/ src/
COPY assets/ assets/

RUN dx build --release

FROM nginx:alpine

COPY --from=builder /app/target/dx/careops-frontend/release/web/public /usr/share/nginx/html
COPY --from=builder /app/assets /usr/share/nginx/html/assets/
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
