# 7aychain Devnet Testing Scripts

Security testing suite for validating 7aychain functionality before testnet deployment.

## Quick Start

```bash
cd devnet/scripts

# Install dependencies
npm install

# Run all tests
npm run all

# Or run individual test suites
npm run crypto    # Cryptographer tests
npm run security  # Security auditor tests
npm run cybersec  # Cybersecurity ops tests
```

## Test Suites

### Cryptographer Tests (`crypto-tests.js`)

Tests cryptographic primitives and protocols:

- **Commitment-Reveal Scheme**: Validates binding property and timing windows
- **Merkle Proofs**: Tree construction, proof generation, verification
- **Nullifier Uniqueness**: Prevents double-spend attacks
- **Secret Sharing**: Shamir 3-of-5 split and reconstruction
- **Hash Benchmarks**: Blake2-256 performance

```bash
npm run crypto
# Or with custom endpoint:
WS_ENDPOINT=ws://localhost:9944 npm run crypto
```

### Security Auditor Tests (`security-audit.js`)

Tests security boundaries and access control:

- **Dispute Resolution**: Non-validator disputes, evidence limits, timing
- **Validator Slashing**: Slash paths, calculations, deferrals
- **Capability Escalation**: Permission delegation limits
- **Storage Bounds**: Collection limits, exhaustion resistance
- **Permission Boundaries**: Origin restrictions, sudo operations

```bash
npm run security
```

### Cybersecurity Ops Tests (`cybersec-ops.js`)

Tests operational security and monitoring:

- **Bot Detection**: Behavior pattern analysis, classification
- **Sybil Attack Detection**: Device attestation, correlated behavior
- **Fraud Proof Generation**: RSSI validation, Z-score calculation
- **Network Partition**: Simulation scenarios, recovery
- **Cluster Health**: Octopus monitoring, heartbeats

```bash
npm run cybersec
```

## Hybrid Devnet

For real device scanning, run Alice natively while other nodes use Docker with mock scanning.

### Native Alice with Real Scanning

```bash
# Build native binary first
cd ../..
cargo build --release --package seveny-node

# Run Alice with real WiFi/Bluetooth scanning
./scripts/run-native-alice.sh

# Options:
./scripts/run-native-alice.sh --mock           # Use mock scanner instead
./scripts/run-native-alice.sh --scan-interval 5 # 5 second scan interval
./scripts/run-native-alice.sh --pos 100 200 0  # Set position
./scripts/run-native-alice.sh --purge          # Clear node data
```

### Docker Nodes (Mock Scanning)

```bash
cd devnet
docker compose -f docker-compose.hybrid.yml up -d
```

This starts Bob, Charlie, Dave, Eve, and Ferdie in Docker with mock device scanning, connecting to native Alice via `host.docker.internal`.

## Real-Time Monitoring

```bash
./scripts/monitor.sh        # Default 5s refresh
./scripts/monitor.sh 10     # 10s refresh
```

Monitors:
- Node health and peer counts
- Block production and finality
- Device scanning statistics
- Network alerts

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `WS_ENDPOINT` | `ws://127.0.0.1:9944` | WebSocket RPC endpoint |

## Test Matrix

| Test | Online | Offline | Notes |
|------|--------|---------|-------|
| Commitment binding | N/A | ✓ | Pure crypto |
| Merkle proofs | N/A | ✓ | Pure crypto |
| Nullifier uniqueness | N/A | ✓ | Pure crypto |
| Shamir sharing | N/A | ✓ | Pure crypto |
| Dispute audit | ✓ | Pattern | Needs chain |
| Slash verification | ✓ | Pattern | Needs chain |
| Bot detection | N/A | ✓ | Algorithm |
| Fraud proofs | N/A | ✓ | Algorithm |
| Cluster health | ✓ | N/A | Needs chain |
| Metrics | ✓ | N/A | Needs chain |

## Platform Requirements

### macOS (Real Device Scanning)

```bash
# Enable location services for terminal
# System Preferences → Security & Privacy → Privacy → Location Services

# Verify WiFi access
/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport -s
```

### Linux (Real Device Scanning)

```bash
# Install BlueZ
sudo apt install bluez bluetooth

# Add user to bluetooth group
sudo usermod -a -G bluetooth $USER

# Start bluetooth service
sudo systemctl start bluetooth
```

### Docker (Mock Scanning Only)

Docker containers cannot access host WiFi/Bluetooth hardware. Use mock scanner mode or hybrid deployment.
