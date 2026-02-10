#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEVNET_DIR="$(dirname "$SCRIPT_DIR")"

cd "$DEVNET_DIR"

echo "Stopping and removing 7aychain devnet containers and volumes..."
docker compose down -v

echo "Removing built images..."
docker compose down --rmi local 2>/dev/null || true

echo "Devnet reset complete. All data has been purged."
