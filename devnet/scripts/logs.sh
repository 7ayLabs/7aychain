#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEVNET_DIR="$(dirname "$SCRIPT_DIR")"

cd "$DEVNET_DIR"

SERVICE=${1:-}

if [ -n "$SERVICE" ]; then
    docker compose logs -f "$SERVICE"
else
    docker compose logs -f
fi
