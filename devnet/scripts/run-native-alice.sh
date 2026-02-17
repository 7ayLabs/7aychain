#!/bin/bash
# Native Alice Runner for Real Device Scanning
# Use this script to run Alice natively with real WiFi/Bluetooth scanning
# while other nodes run in Docker with mock scanning

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BINARY="$PROJECT_ROOT/target/release/seveny-node"
DATA_DIR="$PROJECT_ROOT/target/alice-data"

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
    echo "   Native Alice - Real Device Scanning"
    echo -e "${NC}"
}

check_binary() {
    if [ ! -f "$BINARY" ]; then
        echo -e "${YELLOW}Binary not found. Building...${NC}"
        cd "$PROJECT_ROOT"
        cargo build --release --package seveny-node
    fi
}

check_macos_permissions() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo -e "${YELLOW}macOS detected. Checking permissions...${NC}"

        # Check location services
        if ! defaults read /var/db/locationd/clients.plist 2>/dev/null | grep -q "Terminal"; then
            echo -e "${RED}Warning: Location Services may not be enabled for Terminal.${NC}"
            echo "Enable: System Preferences → Security & Privacy → Privacy → Location Services"
        fi

        # Test WiFi scanning
        if /System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport -s &>/dev/null; then
            echo -e "${GREEN}✓ WiFi scanning available${NC}"
        else
            echo -e "${RED}✗ WiFi scanning unavailable. Check permissions.${NC}"
        fi
    fi
}

cleanup() {
    echo -e "\n${YELLOW}Shutting down Alice...${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Parse arguments
SCANNER_MODE="latency"
SCAN_INTERVAL=10
POS_X=0
POS_Y=0
POS_Z=0

while [[ $# -gt 0 ]]; do
    case $1 in
        --mock)
            SCANNER_MODE="mock"
            shift
            ;;
        --scan-interval)
            SCAN_INTERVAL="$2"
            shift 2
            ;;
        --pos)
            POS_X="$2"
            POS_Y="$3"
            POS_Z="$4"
            shift 4
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
check_macos_permissions

echo -e "${GREEN}Starting Alice with:${NC}"
echo "  Scanner Mode: $SCANNER_MODE"
echo "  Scan Interval: ${SCAN_INTERVAL}s"
echo "  Position: ($POS_X, $POS_Y, $POS_Z)"
echo ""

mkdir -p "$DATA_DIR"

exec "$BINARY" \
    --alice \
    --validator \
    --chain=local \
    --base-path="$DATA_DIR" \
    --rpc-cors=all \
    --rpc-external \
    --rpc-methods=unsafe \
    --node-key=0000000000000000000000000000000000000000000000000000000000000001 \
    --scanner-mode="$SCANNER_MODE" \
    --scan-interval="$SCAN_INTERVAL" \
    --scanner-pos-x="$POS_X" \
    --scanner-pos-y="$POS_Y" \
    --scanner-pos-z="$POS_Z"
