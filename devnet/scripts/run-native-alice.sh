#!/bin/bash
# Native Alice Runner for External Device Scan Bridging
# Use this script to run Alice natively while another local tool publishes
# scan observations into a JSON bridge file consumed by the node.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BINARY="$PROJECT_ROOT/target/release/seveny-node"
DATA_DIR="$PROJECT_ROOT/target/alice-data"
SCAN_DIR="$PROJECT_ROOT/devnet/state"
EXTERNAL_SCAN_FILE="$SCAN_DIR/alice-scan.json"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

print_banner() {
    echo -e "${GREEN}"
    echo "  _____ ___  _  _  ___ _  _   _   ___ _  _"
    echo " |___  / _ \| || |/ __| || | /_\ |_ _| \| |"
    echo "    / / (_) | __ | (__| __ |/ _ \ | || .\` |"
    echo "   /_/ \__\_\_||_|\___|_||_/_/ \_\___|_|\_|"
    echo ""
    echo "   Native Alice - External Scan Bridge"
    echo -e "${NC}"
}

check_binary() {
    if [ ! -f "$BINARY" ]; then
        echo -e "${YELLOW}Binary not found. Building...${NC}"
        cd "$PROJECT_ROOT"
        cargo build --release --package seveny-node
    fi
}

print_bridge_hint() {
    echo -e "${YELLOW}External scan bridge file:${NC} $EXTERNAL_SCAN_FILE"
    echo "Populate it with:"
    echo "  python3 devnet/scripts/publish_external_scan.py --sample --output \"$EXTERNAL_SCAN_FILE\""
}

cleanup() {
    echo -e "\n${YELLOW}Shutting down Alice...${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Parse arguments
SCANNER_MODE="external"
SCAN_INTERVAL=10
MAX_SCAN_AGE=30
POS_X=0
POS_Y=0
POS_Z=0
RPC_METHODS="safe"
RPC_EXTERNAL=0

while [[ $# -gt 0 ]]; do
    case $1 in
        --mock)
            SCANNER_MODE="mock"
            shift
            ;;
        --latency)
            SCANNER_MODE="latency"
            shift
            ;;
        --external-scan-file)
            EXTERNAL_SCAN_FILE="$2"
            shift 2
            ;;
        --scan-interval)
            SCAN_INTERVAL="$2"
            shift 2
            ;;
        --max-scan-age)
            MAX_SCAN_AGE="$2"
            shift 2
            ;;
        --pos)
            POS_X="$2"
            POS_Y="$3"
            POS_Z="$4"
            shift 4
            ;;
        --rpc-external)
            RPC_EXTERNAL=1
            shift
            ;;
        --unsafe-rpc)
            RPC_METHODS="unsafe"
            shift
            ;;
        --purge)
            echo -e "${YELLOW}Purging Alice data...${NC}"
            rm -rf "$DATA_DIR"
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

print_banner
check_binary
print_bridge_hint

echo -e "${GREEN}Starting Alice with:${NC}"
echo "  Scanner Mode: $SCANNER_MODE"
echo "  Scan Interval: ${SCAN_INTERVAL}s"
echo "  Max Scan Age: ${MAX_SCAN_AGE}s"
echo "  Position: ($POS_X, $POS_Y, $POS_Z)"
if [[ "$SCANNER_MODE" == "external" ]]; then
    echo "  External Scan File: $EXTERNAL_SCAN_FILE"
fi
echo ""

mkdir -p "$DATA_DIR"
mkdir -p "$SCAN_DIR"

ARGS=(
    --alice
    --validator
    --chain=local
    --base-path="$DATA_DIR"
    --rpc-cors=all
    --rpc-methods="$RPC_METHODS"
    --node-key=0000000000000000000000000000000000000000000000000000000000000001
    --scanner-mode="$SCANNER_MODE"
    --scan-interval="$SCAN_INTERVAL"
    --max-scan-age="$MAX_SCAN_AGE"
    --scanner-pos-x="$POS_X"
    --scanner-pos-y="$POS_Y"
    --scanner-pos-z="$POS_Z"
)

if [[ "$RPC_EXTERNAL" -eq 1 ]]; then
    ARGS+=(--rpc-external)
fi

if [[ "$SCANNER_MODE" == "external" ]]; then
    ARGS+=(--external-scan-file="$EXTERNAL_SCAN_FILE")
fi

exec "$BINARY" "${ARGS[@]}"
