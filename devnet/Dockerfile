FROM docker.io/library/rust:latest AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    clang \
    libclang-dev \
    protobuf-compiler \
    libdbus-1-dev \
    pkg-config \
    cmake \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown && \
    rustup component add rust-src

WORKDIR /7aychain

COPY rust-toolchain.toml ./
COPY Cargo.toml Cargo.lock ./
COPY primitives/Cargo.toml primitives/Cargo.toml
COPY runtime/Cargo.toml runtime/Cargo.toml
COPY node/Cargo.toml node/Cargo.toml
COPY pallets pallets

RUN mkdir -p primitives/src runtime/src node/src && \
    echo "fn main() {}" > node/src/main.rs && \
    echo "pub fn dummy() {}" > primitives/src/lib.rs && \
    echo "pub fn dummy() {}" > runtime/src/lib.rs && \
    echo "fn main() {}" > node/build.rs

RUN cargo build --release --package seveny-node || true

COPY . .

ENV CARGO_INCREMENTAL=0
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# Clean workspace crates so Cargo recompiles from real sources
RUN cargo clean -p seveny-primitives -p seveny-runtime -p seveny-node --release 2>/dev/null || true

RUN cargo build --release --package seveny-node

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
