# 7aychain

**Substrate-based implementation of the 7ay Proof of Presence (PoP) Protocol v0.7.6**

[![Build Status](https://img.shields.io/github/actions/workflow/status/7ayLabs/7aychain/ci.yml?branch=main)](https://github.com/7ayLabs/7aychain/actions)
[![License](https://img.shields.io/badge/license-BUSL--1.1-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)

## Overview

7aychain is a standalone Layer 1 blockchain implementing the [7ay Proof of Presence Protocol](https://github.com/7ayLabs/7ay-presence). It provides on-chain actor presence certification through epoch-bound, validator-quorum finalization.

### Key Features

- **Presence State Machine**: Deterministic state transitions (None → Declared → Validated → Finalized)
- **Epoch-Bound Validation**: Temporal boundaries ensuring presence validity within defined periods
- **Validator Quorum**: Consensus-based validation with configurable quorum requirements
- **Economic Security**: Stake-based validator participation with slashing mechanisms
- **Protocol Invariants**: 78 enforced invariants (INV1-78) ensuring system correctness

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        7aychain Node                            │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Application Layer                        ││
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           ││
│  │  │Presence │ │ Epoch   │ │Validator│ │ Dispute │           ││
│  │  │ Pallet  │ │ Pallet  │ │ Pallet  │ │ Pallet  │           ││
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘           ││
│  └─────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    Domain Layer                             ││
│  │  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐        ││
│  │  │PresenceCore  │ │ EpochEngine  │ │ ValidatorSet │        ││
│  │  │  (INV1-13)   │ │  (Lifecycle) │ │ (INV46-49)   │        ││
│  │  └──────────────┘ └──────────────┘ └──────────────┘        ││
│  └─────────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                 Infrastructure Layer                        ││
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐   ││
│  │  │Storage │ │Crypto  │ │Network │ │ RPC    │ │Telemetry│  ││
│  │  └────────┘ └────────┘ └────────┘ └────────┘ └────────┘   ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Protocol Layers

| Layer | Description | Invariants |
|-------|-------------|------------|
| **Presence** | State machine and validation | INV1-13 |
| **Epoch** | Temporal boundaries | Lifecycle |
| **Validator** | Quorum and staking | INV46-49 |
| **Dispute** | Resolution and slashing | INV48 |
| **Recovery** | Validator recovery | INV57-60 |
| **Security** | Chain binding, rate limiting | INV43-45 |

## Getting Started

### Prerequisites

- Rust 1.75+ (stable)
- Cargo
- Git

### Build

```bash
# Clone the repository
git clone https://github.com/7ayLabs/7aychain.git
cd 7aychain

# Build in release mode
cargo build --release

# Run tests
cargo test --all

# Check linting
cargo clippy --all -- -D warnings
```

### Development

```bash
# Run local development node
./target/release/7aychain --dev

# Run with detailed logging
RUST_LOG=debug ./target/release/7aychain --dev
```

## Project Structure

```
7aychain/
├── primitives/          # Core types, constants, and shared traits
├── pallets/             # FRAME pallets (presence, epoch, validator, dispute)
├── runtime/             # Runtime configuration and WASM build
├── node/                # Node implementation and CLI
└── client/              # Client-side RPC extensions
```

## Protocol Constants

Key protocol parameters derived from the [PoP Specification](https://github.com/7ayLabs/7ay-presence):

| Constant | Value | Invariant |
|----------|-------|-----------|
| Minimum Validators | 5 | INV46 |
| Max Stake Ratio | 33% | INV47 |
| Recovery Quorum | 80% | INV57 |
| Emergency Upgrade Quorum | 80% | INV60 |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## Security

See [SECURITY.md](SECURITY.md) for security policy and vulnerability reporting.

## Documentation

- [Protocol Specification](https://github.com/7ayLabs/7ay-presence)
- [API Documentation](https://docs.7aylabs.com)

## License

This project is licensed under the Business Source License 1.1 - see the [LICENSE](LICENSE) file for details.

---

Built with [Substrate](https://substrate.io/) by [7ayLabs](https://7aylabs.com)
