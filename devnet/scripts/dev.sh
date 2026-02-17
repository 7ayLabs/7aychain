#!/usr/bin/env bash
# Start a single-node devnet with instant seal.
# Blocks are produced ONLY when you submit extrinsics.
#
# Usage:  ./scripts/dev.sh          # Docker (build + start)
#         ./scripts/dev.sh native   # Native binary (no Docker)
#         ./scripts/dev.sh stop     # Stop the container
#         ./scripts/dev.sh reset    # Stop + clear chain state

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEVNET_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_ROOT="$(cd "$DEVNET_DIR/.." && pwd)"
cd "$DEVNET_DIR"

case "${1:-docker}" in
  native)
    echo "Starting native instant-seal node..."
    cargo build --release --package seveny-node --manifest-path "$PROJECT_ROOT/Cargo.toml" 2>&1 | tail -3
    "$PROJECT_ROOT/target/release/seveny-node" \
      --dev \
      --sealing=instant \
      --rpc-cors=all \
      --rpc-methods=unsafe \
      --scanner-mode=mock \
      --mock-devices=15 \
      --tmp
    ;;
  stop)
    docker compose -f docker-compose.dev.yml down
    echo "Stopped."
    ;;
  reset)
    docker compose -f docker-compose.dev.yml down -v
    echo "Stopped and cleared chain state."
    ;;
  docker|*)
    docker compose -f docker-compose.dev.yml up -d --build
    echo ""
    echo "  Instant-seal devnet running on ws://127.0.0.1:9944"
    echo "  Blocks produced ONLY on extrinsic submission."
    echo ""
    echo "  Next: python3 scripts/laud-cli.py"
    echo ""
    ;;
esac
