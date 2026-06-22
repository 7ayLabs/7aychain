#!/usr/bin/env bash
# Start the local carrier bridge + softphone bridge watchers for the telecom devnet.

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
TRUNK_CONFIG="${TRUNK_CONFIG:-}"
AUTO_RELOAD="${AUTO_RELOAD:-true}"
RELOAD_METHOD="${RELOAD_METHOD:-cli}"
WEBRTC="${WEBRTC:-true}"
PROVISIONING_PORT="${PROVISIONING_PORT:-8090}"

cleanup() {
  kill "${CARRIER_PID:-0}" "${SOFTPHONE_PID:-0}" "${ESIM_PID:-0}" \
       "${PROVISIONING_PID:-0}" 2>/dev/null || true
}

trap cleanup EXIT INT TERM

if ! "$PYTHON_BIN" -c "import substrateinterface, qrcode" >/dev/null 2>&1; then
  echo "Missing Python dependencies: substrate-interface and/or qrcode"
  echo "Install it with:"
  if [ -x "$DEVNET_DIR/.venv/bin/pip" ]; then
    echo "  $DEVNET_DIR/.venv/bin/pip install -r devnet/scripts/requirements.txt"
  else
    echo "  python3 -m venv devnet/.venv"
    echo "  devnet/.venv/bin/pip install -r devnet/scripts/requirements.txt"
  fi
  exit 1
fi

mkdir -p "$(dirname "$BRIDGE_OUTPUT")"

# Generate TLS certs for WebRTC if needed
if [ "$WEBRTC" = "true" ]; then
  bash devnet/telecom/webrtc/generate-cert.sh devnet/telecom/certs
fi

# Carrier bridge (chain watcher)
"$PYTHON_BIN" devnet/scripts/carrier_bridge.py \
  --watch \
  --catalog "$CATALOG_PATH" \
  --output "$BRIDGE_OUTPUT" &
CARRIER_PID=$!

# Softphone bridge (config generator)
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

# eSIM bridge
"$PYTHON_BIN" devnet/scripts/carrier_esim_bridge.py \
  --watch \
  --snapshot "$BRIDGE_OUTPUT" &
ESIM_PID=$!

# Provisioning API
"$PYTHON_BIN" devnet/scripts/carrier_provisioning_api.py \
  --port "$PROVISIONING_PORT" \
  --bridge-json "$BRIDGE_OUTPUT" \
  --catalog-json "$CATALOG_PATH" \
  --sip-domain "$SIP_DOMAIN" \
  --sip-port "$SIP_PORT" &
PROVISIONING_PID=$!

# Detect LAN IP for convenience
LAN_IP=$(ifconfig 2>/dev/null | grep 'inet ' | grep -v '127.0.0.1' | head -1 | awk '{print $2}' || echo "127.0.0.1")

echo ""
echo "=== 7AY Telecom Devnet ==="
echo "Carrier bridge PID: $CARRIER_PID"
echo "Softphone bridge PID: $SOFTPHONE_PID"
echo "eSIM bridge PID: $ESIM_PID"
echo "Provisioning API PID: $PROVISIONING_PID"
echo ""
echo "Snapshot:      $BRIDGE_OUTPUT"
echo "Provisioning:  devnet/telecom/generated/subscribers.softphone.json"
echo "eSIM gallery:  devnet/telecom/esim/index.html"
echo ""
echo "Provisioning API: http://${LAN_IP}:${PROVISIONING_PORT}/api/v1/subscribers"
echo "Health check:     http://${LAN_IP}:${PROVISIONING_PORT}/api/v1/health"

if [ "$WEBRTC" = "true" ]; then
  echo "WebRTC dialer:    https://${LAN_IP}:8089/static/webrtc/index.html"
fi

if [ -n "$TRUNK_CONFIG" ]; then
  echo "SIP trunk:        $TRUNK_CONFIG"
fi

echo ""
echo "Python:            $PYTHON_BIN"
echo "Dependencies:      $DEVNET_DIR/.venv/bin/pip install -r devnet/scripts/requirements.txt"

wait
