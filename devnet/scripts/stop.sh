#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEVNET_DIR="$(dirname "$SCRIPT_DIR")"

cd "$DEVNET_DIR"

echo "Stopping 7aychain devnet..."
docker compose down

echo "Devnet stopped."
