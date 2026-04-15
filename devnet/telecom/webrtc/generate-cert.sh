#!/usr/bin/env bash
# Generate self-signed TLS certificate for Asterisk WebSocket (WSS).
# Required because browsers mandate WSS (not plain WS) for WebRTC.

set -euo pipefail

CERT_DIR="${1:-devnet/telecom/certs}"
mkdir -p "$CERT_DIR"

CERT="$CERT_DIR/cert.pem"
KEY="$CERT_DIR/key.pem"

if [ -f "$CERT" ] && [ -f "$KEY" ]; then
    echo "TLS cert already exists at $CERT_DIR, skipping generation."
    exit 0
fi

# Try mkcert first (locally-trusted certs)
if command -v mkcert >/dev/null 2>&1; then
    echo "Using mkcert for locally-trusted certificate..."
    mkcert -cert-file "$CERT" -key-file "$KEY" \
        localhost 127.0.0.1 ::1 \
        "$(hostname)" "$(hostname).local" \
        "*.local"
    echo "Done. Trust store updated by mkcert."
    exit 0
fi

# Fallback to openssl self-signed
echo "Using openssl for self-signed certificate..."
LAN_IP=$(ifconfig 2>/dev/null | grep 'inet ' | grep -v '127.0.0.1' | head -1 | awk '{print $2}' || echo "127.0.0.1")

openssl req -x509 -newkey rsa:2048 -nodes \
    -keyout "$KEY" -out "$CERT" \
    -days 365 \
    -subj "/CN=7aychain-devnet" \
    -addext "subjectAltName=DNS:localhost,IP:127.0.0.1,IP:$LAN_IP"

echo "Self-signed cert created at $CERT_DIR"
echo "Note: browsers will show a security warning. Accept it for local testing."
