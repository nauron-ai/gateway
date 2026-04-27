# syntax=docker/dockerfile:1.7

FROM rust:1.95.0-slim-bookworm AS builder

ENV CARGO_HOME=/workspace/.app_cache/cargo \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse \
    CARGO_TARGET_DIR=/workspace/.app_cache/target \
    SQLX_OFFLINE=true

WORKDIR /workspace

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        clang \
        cmake \
        libcurl4-openssl-dev \
        libssl-dev \
        libsasl2-dev \
        libzstd-dev \
        pkg-config \
        zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

RUN mkdir -p src \
    && echo "fn main() {}" > src/main.rs

RUN --mount=type=cache,target=/workspace/.app_cache/cargo,sharing=locked \
    --mount=type=cache,target=/workspace/.app_cache/target,sharing=locked \
    cargo build --release --locked

COPY migrations migrations
COPY .sqlx .sqlx
COPY src src

RUN --mount=type=cache,target=/workspace/.app_cache/cargo,sharing=locked \
    --mount=type=cache,target=/workspace/.app_cache/target,sharing=locked \
    touch src/main.rs \
    && cargo build --release --locked \
    && cp /workspace/.app_cache/target/release/gateway /workspace/gateway

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/gateway /usr/local/bin/gateway

ENTRYPOINT ["gateway"]
