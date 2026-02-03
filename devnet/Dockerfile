FROM docker.io/paritytech/ci-unified:bullseye-1.81.0 AS builder

WORKDIR /7aychain

COPY . .

RUN cargo build --release

FROM docker.io/library/debian:bullseye-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        libssl1.1 \
        ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 -U -s /bin/sh -d /node node

COPY --from=builder /7aychain/target/release/seveny-node /usr/local/bin/

RUN chmod +x /usr/local/bin/seveny-node && \
    chown node:node /usr/local/bin/seveny-node

USER node

RUN mkdir -p /node/data

EXPOSE 9944 9933 30333

VOLUME ["/node/data"]

ENTRYPOINT ["/usr/local/bin/seveny-node"]
