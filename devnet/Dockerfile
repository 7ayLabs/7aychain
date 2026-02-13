FROM docker.io/library/rust:latest AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    libclang-dev \
    protobuf-compiler \
    libdbus-1-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Add wasm target and rust-src for WASM runtime compilation
RUN rustup target add wasm32-unknown-unknown && \
    rustup component add rust-src

WORKDIR /7aychain

COPY . .

RUN cargo build --release

FROM docker.io/library/ubuntu:24.04

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libssl3 \
        libdbus-1-3 \
        ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m -U -s /bin/sh -d /node node

COPY --from=builder /7aychain/target/release/seveny-node /usr/local/bin/

RUN chmod +x /usr/local/bin/seveny-node && \
    chown node:node /usr/local/bin/seveny-node

USER node

RUN mkdir -p /node/data

EXPOSE 9944 9933 30333

VOLUME ["/node/data"]

ENTRYPOINT ["/usr/local/bin/seveny-node"]
