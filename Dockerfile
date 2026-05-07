ARG RUST_VERSION=1.94
ARG APP_NAME=schedule_bot


FROM rust:${RUST_VERSION}-slim AS builder
ARG APP_NAME

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release


FROM debian:trixie-slim
ARG APP_NAME

RUN apt-get update && apt-get install -y \
    libssl3 \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/${APP_NAME} /usr/local/bin/app

CMD ["app"]
