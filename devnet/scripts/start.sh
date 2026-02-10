#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEVNET_DIR="$(dirname "$SCRIPT_DIR")"

cd "$DEVNET_DIR"

echo "Building and starting 7aychain devnet..."
docker compose up -d --build

echo ""
echo "7aychain devnet is starting up..."
echo ""
echo "Endpoints:"
echo "  Alice (Validator 1): ws://localhost:9944"
echo "  Bob   (Validator 2): ws://localhost:9945"
echo "  Charlie (Validator 3): ws://localhost:9946"
echo ""
echo "Use 'docker compose logs -f' to view logs"
echo "Use './scripts/stop.sh' to stop the network"
