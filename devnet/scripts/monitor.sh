#!/bin/bash
# 7aychain Devnet Monitor
# Real-time monitoring of devnet nodes

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Node endpoints
NODES=(
    "Alice:localhost:9944"
    "Bob:localhost:9945"
    "Charlie:localhost:9946"
    "Dave:localhost:9947"
    "Eve:localhost:9948"
    "Ferdie:localhost:9949"
)

REFRESH_INTERVAL=${1:-5}

print_banner() {
    clear
    echo -e "${CYAN}"
    echo "  _____ ___  _  _  ___ _  _   _   ___ _  _"
    echo " |___  / _ \| || |/ __| || | /_\ |_ _| \| |"
    echo "    / / (_) | __ | (__| __ |/ _ \ | || .\` |"
    echo "   /_/ \__\_\_||_|\___|_||_/_/ \_\___|_|\_|"
    echo ""
    echo "   Devnet Monitor - Refresh: ${REFRESH_INTERVAL}s"
    echo -e "${NC}"
    echo "═══════════════════════════════════════════════════════════"
}

query_rpc() {
    local endpoint=$1
    local method=$2
    local params=${3:-"[]"}

    curl -s -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"${method}\",\"params\":${params}}" \
        "http://${endpoint}" 2>/dev/null | jq -r '.result // empty'
}

check_node_health() {
    local name=$1
    local endpoint=$2

    local health=$(query_rpc "$endpoint" "system_health")

    if [ -z "$health" ]; then
        echo -e "  ${RED}✗${NC} ${name}: Offline"
        return 1
    fi

    local peers=$(echo "$health" | jq -r '.peers // 0')
    local syncing=$(echo "$health" | jq -r '.isSyncing // false')

    local status="${GREEN}✓${NC}"
    local sync_status=""

    if [ "$syncing" = "true" ]; then
        status="${YELLOW}↻${NC}"
        sync_status=" (syncing)"
    fi

    if [ "$peers" -lt 3 ]; then
        status="${YELLOW}!${NC}"
    fi

    echo -e "  ${status} ${name}: ${peers} peers${sync_status}"
    return 0
}

get_block_info() {
    local endpoint=$1

    local header=$(query_rpc "$endpoint" "chain_getHeader")
    local finalized=$(query_rpc "$endpoint" "chain_getFinalizedHead")

    if [ -z "$header" ]; then
        echo "N/A"
        return
    fi

    local block_num=$(echo "$header" | jq -r '.number // "0x0"')
    local block_dec=$(printf '%d' "$block_num")

    # Get finalized block number
    if [ -n "$finalized" ]; then
        local fin_header=$(curl -s -H "Content-Type: application/json" \
            -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"chain_getHeader\",\"params\":[\"${finalized}\"]}" \
            "http://${endpoint}" 2>/dev/null | jq -r '.result // empty')

        if [ -n "$fin_header" ]; then
            local fin_num=$(echo "$fin_header" | jq -r '.number // "0x0"')
            local fin_dec=$(printf '%d' "$fin_num")
            echo "Block: $block_dec | Finalized: $fin_dec | Lag: $((block_dec - fin_dec))"
            return
        fi
    fi

    echo "Block: $block_dec"
}

get_device_stats() {
    local endpoint=$1

    # Try custom RPC if available
    local stats=$(query_rpc "$endpoint" "deviceScanner_getStatistics")

    if [ -n "$stats" ]; then
        local active=$(echo "$stats" | jq -r '.activeDevices // 0')
        local total=$(echo "$stats" | jq -r '.totalDetected // 0')
        echo "Devices: ${active} active / ${total} total"
    else
        echo "Devices: N/A (RPC not available)"
    fi
}

show_docker_status() {
    echo ""
    echo "Docker Containers:"
    echo "─────────────────────────────────────────────────────────"

    docker ps --filter "name=seveny-" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" 2>/dev/null || \
        echo "  Docker not available or no containers running"
}

show_network_status() {
    echo ""
    echo "P2P Network:"
    echo "─────────────────────────────────────────────────────────"

    # Get peer info from first available node
    for node_info in "${NODES[@]}"; do
        IFS=':' read -r name host port <<< "$node_info"
        local peers=$(query_rpc "${host}:${port}" "system_peers")

        if [ -n "$peers" ]; then
            local peer_count=$(echo "$peers" | jq 'length')
            echo "  Connected peers: $peer_count"

            echo "$peers" | jq -r '.[] | "    → \(.peerId[0:16])... (\(.bestNumber))"' 2>/dev/null | head -5
            break
        fi
    done
}

show_alerts() {
    echo ""
    echo "Alerts:"
    echo "─────────────────────────────────────────────────────────"

    local alerts=0

    # Check finality lag
    for node_info in "${NODES[@]}"; do
        IFS=':' read -r name host port <<< "$node_info"
        local header=$(query_rpc "${host}:${port}" "chain_getHeader")

        if [ -n "$header" ]; then
            local block_num=$(echo "$header" | jq -r '.number // "0x0"')
            local block_dec=$((block_num))

            local finalized=$(query_rpc "${host}:${port}" "chain_getFinalizedHead")
            if [ -n "$finalized" ]; then
                local fin_header=$(curl -s -H "Content-Type: application/json" \
                    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"chain_getHeader\",\"params\":[\"${finalized}\"]}" \
                    "http://${host}:${port}" 2>/dev/null | jq -r '.result // empty')

                if [ -n "$fin_header" ]; then
                    local fin_num=$(echo "$fin_header" | jq -r '.number // "0x0"')
                    local fin_dec=$((fin_num))
                    local lag=$((block_dec - fin_dec))

                    if [ "$lag" -gt 3 ]; then
                        echo -e "  ${YELLOW}⚠${NC}  High finality lag: $lag blocks"
                        alerts=$((alerts + 1))
                    fi
                fi
            fi
            break
        fi
    done

    # Check low peer counts
    for node_info in "${NODES[@]}"; do
        IFS=':' read -r name host port <<< "$node_info"
        local health=$(query_rpc "${host}:${port}" "system_health")

        if [ -n "$health" ]; then
            local peers=$(echo "$health" | jq -r '.peers // 0')
            if [ "$peers" -lt 3 ]; then
                echo -e "  ${YELLOW}⚠${NC}  ${name}: Low peer count ($peers)"
                alerts=$((alerts + 1))
            fi
        fi
    done

    if [ "$alerts" -eq 0 ]; then
        echo -e "  ${GREEN}✓${NC} No alerts"
    fi
}

main_loop() {
    while true; do
        print_banner

        echo ""
        echo "Node Health:"
        echo "─────────────────────────────────────────────────────────"

        for node_info in "${NODES[@]}"; do
            IFS=':' read -r name host port <<< "$node_info"
            check_node_health "$name" "${host}:${port}"
        done

        echo ""
        echo "Chain Status:"
        echo "─────────────────────────────────────────────────────────"

        # Get info from first available node
        for node_info in "${NODES[@]}"; do
            IFS=':' read -r name host port <<< "$node_info"
            local info=$(get_block_info "${host}:${port}")
            if [ "$info" != "N/A" ]; then
                echo "  $info"
                break
            fi
        done

        # Device stats
        for node_info in "${NODES[@]}"; do
            IFS=':' read -r name host port <<< "$node_info"
            local devices=$(get_device_stats "${host}:${port}")
            if [[ "$devices" != *"N/A"* ]]; then
                echo "  $devices"
                break
            fi
        done

        show_docker_status
        show_network_status
        show_alerts

        echo ""
        echo "─────────────────────────────────────────────────────────"
        echo "Press Ctrl+C to exit | Refreshing in ${REFRESH_INTERVAL}s..."

        sleep "$REFRESH_INTERVAL"
    done
}

# Check dependencies
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required. Install with: brew install jq"
    exit 1
fi

if ! command -v curl &> /dev/null; then
    echo "Error: curl is required"
    exit 1
fi

# Run monitor
main_loop
