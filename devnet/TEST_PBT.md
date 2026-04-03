# Testing Presence-Based Triangulation (PBT)

This guide explains how to test the new PBT architecture that replaces direct WiFi/Bluetooth scanning. For live devnet ingestion, the node now consumes an external JSON scan bridge in `external` scanner mode.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Presence-Based Triangulation                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. POSITION CLAIM          2. WITNESS ATTESTATION              │
│  ┌──────────────┐           ┌─────────────────────┐             │
│  │ Node claims  │           │ Validators attest   │             │
│  │ position     │───────▶   │ via network latency │             │
│  │ (x, y, z)    │           │ (RTT measurement)   │             │
│  └──────────────┘           └─────────────────────┘             │
│                                      │                           │
│                                      ▼                           │
│                        3. TRIANGULATION                          │
│                   ┌─────────────────────┐                        │
│                   │ Calculate position  │                        │
│                   │ from witness circles│                        │
│                   │ (weighted centroid) │                        │
│                   └─────────────────────┘                        │
│                              │                                   │
│                              ▼                                   │
│                    4. VERIFICATION                               │
│               ┌───────────────────────┐                          │
│               │ Compare claimed vs    │                          │
│               │ triangulated position │                          │
│               │ within tolerance      │                          │
│               └───────────────────────┘                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

### 1. Rebuild Docker Image

```bash
cd /Users/mac/Desktop/Zaid/empresa/proyectos/repos/7aychain/devnet

# Clean old containers and images
docker compose down -v
docker rmi seveny-node:latest 2>/dev/null || true

# Rebuild with new code
docker compose build --no-cache
```

### 2. Start the Devnet

```bash
docker compose up -d
```

### 3. Verify Nodes Are Running

```bash
docker compose ps
docker compose logs -f alice  # Watch logs
```

### 4. Publish External Scan Data For Alice

```bash
cd /Users/mac/Desktop/Zaid/empresa/proyectos/repos/7aychain
python3 devnet/scripts/publish_external_scan.py --sample
```

This writes `devnet/state/alice-scan.json`, which the native Alice process reads when started with `--scanner-mode=external`.

### 5. Run PBT Test Script

```bash
cd scripts
npm install
node test-pbt.js
```

## Manual Testing via Polkadot.js Apps

1. Open https://polkadot.js.org/apps
2. Connect to `ws://127.0.0.1:9944`
3. Go to **Developer** → **Extrinsics**

### Test 1: Set Validator Position

```
presence.setValidatorPosition(validator, position)
- validator: <validator H256>
- position: { x: 0, y: 0, z: 0 }
```

### Test 2: Claim Position

```
presence.claimPosition(epoch, position)
- epoch: 1
- position: { x: 0, y: 0, z: 0 }
```

### Test 3: Submit Witness Attestation

```
presence.submitWitnessAttestation(target, epoch, latency_ms, direct_connection)
- target: <actor H256>
- epoch: 1
- latency_ms: 5
- direct_connection: true
```

### Test 4: Verify Position

```
presence.verifyPosition(target, epoch)
- target: <actor H256>
- epoch: 1
```

## Real-Device Checklist

1. Run Alice natively with `devnet/scripts/run-native-alice.sh --pos 0 0 0`.
2. Keep `scanner-mode=external` and publish a fresh bridge file every time your local device observations change.
3. Use `python3 devnet/scripts/publish_external_scan.py --sample` for smoke tests before wiring a real hardware collector.
4. For real hardware, replace the sample publisher with a local bridge process that writes the same JSON schema to `devnet/state/alice-scan.json`.
5. Ensure new scan payloads arrive more frequently than `--max-scan-age`, otherwise the node will drop them as stale.
6. Confirm Alice logs at least one external scan batch before submitting presence-related extrinsics.
7. Bring up the Docker validators with `docker compose -f docker-compose.hybrid.yml up -d`.
8. Submit validator positions and witness attestations from separate machines or separate network segments.
9. Verify the position only after at least three witnesses have submitted attestations.
10. Clear the bridge with `python3 devnet/scripts/publish_external_scan.py --clear` and confirm no new device-scan inherents are authored.

## External Scan JSON Schema

```json
{
  "generated_at": 1700000000,
  "devices": [
    {
      "mac_hash": "0x1111111111111111111111111111111111111111111111111111111111111111",
      "rssi": -41,
      "signal_type": "wifi",
      "device_type": "iphone",
      "device_name": "alice-phone",
      "frequency": 2412
    }
  ]
}
```

## Query State

### Position Claims
```
presence.positionClaims(epoch, actorId) → PositionClaim
```

### Attestation Count
```
presence.attestationCount(epoch, actorId) → u32
```

### Validator Positions
```
presence.validatorPositions(validatorId) → Position
```

## Expected Events

When running tests, watch for these events:

- `PositionClaimed` - Actor claimed a position
- `WitnessAttestationSubmitted` - Validator attested to another's presence
- `PositionVerified` - Position successfully verified (within tolerance)
- `PositionDisputed` - Position failed verification (outside tolerance)
- `ValidatorPositionUpdated` - Validator updated their position

## Troubleshooting

### "PositionNotClaimed" Error
The target actor must call `claimPosition` before witnesses can attest.

### "ValidatorPositionNotSet" Error
Witnesses must set their own position first via `setValidatorPosition`.

### "InsufficientWitnesses" Error
Need at least 3 witness attestations to verify a position.

### "DuplicateAttestation" Error
A witness can only attest once per epoch per target.

### "SelfAttestation" Error
Validators cannot attest to their own presence.

## Configuration Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| MinWitnessesForVerification | 3 | Minimum attestations to verify |
| PositionToleranceMeters | 1000 | Max deviation for verification |

## Network Latency Distance Formula

```
max_distance_km = (RTT_ms / 2) × 150
```

Where 150 km/ms is the approximate speed of light in fiber.

Example:
- RTT = 10ms → max_distance = 750km
- RTT = 2ms → max_distance = 150km
