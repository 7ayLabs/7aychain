#!/usr/bin/env bash
# Start the fuller 7AY telecom test stack:
# chain watcher + softphone/eSIM bridges + provisioning API + mock SM-DP+ +
# signal-plane service, with optional dockerized SIP edge services.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEVNET_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_ROOT="$(cd "$DEVNET_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

VENV_PYTHON="$DEVNET_DIR/.venv/bin/python3"
PYTHON_BIN="${PYTHON_BIN:-}"

if [ -z "$PYTHON_BIN" ]; then
  if [ -x "$VENV_PYTHON" ]; then
    PYTHON_BIN="$VENV_PYTHON"
  else
    PYTHON_BIN="python3"
  fi
fi

CATALOG_PATH="${CATALOG_PATH:-devnet/state/carrier-catalog.json}"
BRIDGE_OUTPUT="${BRIDGE_OUTPUT:-devnet/state/carrier-bridge.json}"
SIP_DOMAIN="${SIP_DOMAIN:-127.0.0.1}"
SIP_PORT="${SIP_PORT:-5060}"
PROVISIONING_PORT="${PROVISIONING_PORT:-8090}"
SMDP_PORT="${SMDP_PORT:-8091}"
SIGNAL_PLANE_PORT="${SIGNAL_PLANE_PORT:-8092}"
AUTO_SAMPLE_CATALOG="${AUTO_SAMPLE_CATALOG:-true}"
DOCKER_STACK="${DOCKER_STACK:-false}"
TRUNK_CONFIG="${TRUNK_CONFIG:-}"
AUTO_RELOAD="${AUTO_RELOAD:-true}"
RELOAD_METHOD="${RELOAD_METHOD:-cli}"
WEBRTC="${WEBRTC:-true}"

cleanup() {
  kill "${CARRIER_PID:-0}" "${SOFTPHONE_PID:-0}" "${ESIM_PID:-0}" \
       "${PROVISIONING_PID:-0}" "${SMDP_PID:-0}" "${SIGNAL_PID:-0}" 2>/dev/null || true
}

trap cleanup EXIT INT TERM

if ! "$PYTHON_BIN" -c "import substrateinterface, qrcode" >/dev/null 2>&1; then
  echo "Missing Python dependencies: substrate-interface and/or qrcode"
  echo "Install with:"
  if [ -x "$DEVNET_DIR/.venv/bin/pip" ]; then
    echo "  $DEVNET_DIR/.venv/bin/pip install -r devnet/scripts/requirements.txt"
  else
    echo "  python3 -m venv devnet/.venv"
    echo "  devnet/.venv/bin/pip install -r devnet/scripts/requirements.txt"
  fi
  exit 1
fi

mkdir -p "$(dirname "$BRIDGE_OUTPUT")"

LAN_IP=$(ifconfig 2>/dev/null | grep 'inet ' | grep -v '127.0.0.1' | head -1 | awk '{print $2}' || echo "127.0.0.1")
SMDP_PLUS="${SMDP_PLUS:-${LAN_IP}:${SMDP_PORT}}"
SMDP_BASE_URL="${SMDP_BASE_URL:-http://${LAN_IP}:${SMDP_PORT}}"

if [ "$AUTO_SAMPLE_CATALOG" = "true" ]; then
  SAMPLE_ARGS=(--sample --output "$CATALOG_PATH" --smdp-plus "$SMDP_PLUS" --provider-name "7AY Telecom Testnet")
  if [ -n "$TRUNK_CONFIG" ]; then
    SAMPLE_ARGS+=(--trunk-config "$TRUNK_CONFIG")
  fi
  "$PYTHON_BIN" devnet/scripts/publish_carrier_catalog.py "${SAMPLE_ARGS[@]}"
fi

if [ "$WEBRTC" = "true" ]; then
  bash devnet/telecom/webrtc/generate-cert.sh devnet/telecom/certs
fi

if [ "$DOCKER_STACK" = "true" ]; then
  docker compose -f devnet/docker-compose.telecom-stack.yml up -d asterisk kamailio
fi

"$PYTHON_BIN" devnet/scripts/carrier_bridge.py \
  --watch \
  --catalog "$CATALOG_PATH" \
  --output "$BRIDGE_OUTPUT" &
CARRIER_PID=$!

SOFTPHONE_ARGS=(
  --watch
  --snapshot "$BRIDGE_OUTPUT"
  --sip-domain "$SIP_DOMAIN"
  --sip-port "$SIP_PORT"
)

if [ -n "$TRUNK_CONFIG" ]; then
  SOFTPHONE_ARGS+=(--trunk-config "$TRUNK_CONFIG")
fi

if [ "$AUTO_RELOAD" = "true" ]; then
  SOFTPHONE_ARGS+=(--auto-reload --reload-method "$RELOAD_METHOD")
fi

if [ "$WEBRTC" = "true" ]; then
  SOFTPHONE_ARGS+=(--webrtc)
fi

"$PYTHON_BIN" devnet/scripts/carrier_softphone_bridge.py "${SOFTPHONE_ARGS[@]}" &
SOFTPHONE_PID=$!

"$PYTHON_BIN" devnet/scripts/carrier_esim_bridge.py \
  --watch \
  --snapshot "$BRIDGE_OUTPUT" &
ESIM_PID=$!

"$PYTHON_BIN" devnet/scripts/carrier_provisioning_api.py \
  --port "$PROVISIONING_PORT" \
  --bridge-json "$BRIDGE_OUTPUT" \
  --catalog-json "$CATALOG_PATH" \
  --sip-domain "$SIP_DOMAIN" \
  --sip-port "$SIP_PORT" &
PROVISIONING_PID=$!

"$PYTHON_BIN" devnet/scripts/carrier_smdp_server.py \
  --port "$SMDP_PORT" \
  --base-url "$SMDP_BASE_URL" \
  --smdp-plus "$SMDP_PLUS" &
SMDP_PID=$!

"$PYTHON_BIN" devnet/scripts/carrier_signal_plane.py \
  --sample \
  --port "$SIGNAL_PLANE_PORT" &
SIGNAL_PID=$!

echo ""
echo "=== 7AY Telecom Test Stack ==="
echo "Carrier bridge PID:      $CARRIER_PID"
echo "Softphone bridge PID:    $SOFTPHONE_PID"
echo "eSIM bridge PID:         $ESIM_PID"
echo "Provisioning API PID:    $PROVISIONING_PID"
echo "Mock SM-DP+ PID:         $SMDP_PID"
echo "Signal plane PID:        $SIGNAL_PID"
echo ""
echo "Snapshot:                $BRIDGE_OUTPUT"
echo "Catalog:                 $CATALOG_PATH"
echo "eSIM gallery:            devnet/telecom/esim/index.html"
echo "Mock SM-DP+ dashboard:   ${SMDP_BASE_URL}/"
echo "Mock SM-DP+ discovery:   ${SMDP_BASE_URL}/api/v1/discovery"
echo "Provisioning API:        http://${LAN_IP}:${PROVISIONING_PORT}/api/v1/subscribers"
echo "Signal plane:            http://${LAN_IP}:${SIGNAL_PLANE_PORT}/api/v1/summary"

if [ "$DOCKER_STACK" = "true" ]; then
  echo "IMS SIP edge:            sip:${LAN_IP}:5062 via Kamailio"
  echo "Asterisk upstream:       sip:${LAN_IP}:5060"
fi

echo ""
echo "Recommended catalog QR host: ${SMDP_PLUS}"
echo "Python:                       $PYTHON_BIN"

wait
