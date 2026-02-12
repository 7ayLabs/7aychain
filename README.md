[![CI](https://github.com/7ayLabs/7aychain/actions/workflows/ci.yml/badge.svg)](https://github.com/7ayLabs/7aychain/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-BUSL--1.1-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![Substrate](https://img.shields.io/badge/substrate-polkadot--stable2503-blueviolet)](https://github.com/paritytech/polkadot-sdk)

7aychain is a standalone Layer 1 blockchain providing on-chain actor presence certification through epoch-bound, validator-quorum finalization. It implements the [7ay Proof of Presence Protocol](https://github.com/7ayLabs/7ay-presence) with 78 enforced invariants ensuring system correctness.

## Building the source

Building `7aychain` requires a Rust toolchain (1.85+), Clang, and LLVM.

### Hardware Requirements

**Minimum:**
- 4 GB RAM
- 2 CPU cores
- 50 GB SSD storage

**Recommended:**
- 8 GB RAM
- 4 CPU cores
- 100 GB NVMe SSD

### Clone and Build

```shell
git clone https://github.com/7ayLabs/7aychain.git
cd 7aychain
cargo build --release
```

The binary is located at `./target/release/seveny-node`.

### Running Tests

```shell
cargo test --workspace
```

### Linting

```shell
cargo clippy --workspace --all-targets -- -D warnings
```

## Executables

The build produces the following binary:

| Command | Description |
|---------|-------------|
| `seveny-node` | Main blockchain node implementing the PoP protocol. Runs as validator, full node, or archive node. |

## Running `seveny-node`

### Development Mode

By far the most common scenario is running a local development node for testing:

```shell
./target/release/seveny-node --dev
```

This starts a single-node development chain with temporary storage. The development account `Alice` is pre-funded and can be used for testing.

For detailed logging:

```shell
RUST_LOG=debug ./target/release/seveny-node --dev
```

### Full Node

To run a full node connecting to the 7aychain network:

```shell
./target/release/seveny-node \
  --chain mainnet \
  --name "my-node" \
  --base-path /data/7aychain
```

### Validator Node

Running a validator requires staking and registration. See the [Validator Guide](https://docs.7aylabs.com/validators) for setup instructions.

```shell
./target/release/seveny-node \
  --chain mainnet \
  --validator \
  --name "my-validator" \
  --base-path /data/7aychain
```

### Docker

```shell
docker run -d \
  -p 30333:30333 \
  -p 9944:9944 \
  -v /data/7aychain:/data \
  7aylabs/7aychain:latest \
  --chain mainnet \
  --base-path /data
```

## Pallets

7aychain implements 14 custom FRAME pallets organized in three layers:

**Protocol Layer**

| Pallet | Description | Invariants |
|--------|-------------|------------|
| Presence | Core presence state machine (None → Declared → Validated → Finalized) | INV1-13 |
| Epoch | Epoch scheduling and lifecycle management | INV14-18 |
| Validator | Validator registration, staking, and slashing | INV46-49 |
| Dispute | Dispute resolution with evidence and quorum outcomes | INV48 |
| Governance | Capability-based access control with delegation | - |

**Identity Layer**

| Pallet | Description | Invariants |
|--------|-------------|------------|
| Lifecycle | Actor registration and key destruction | INV76-78 |
| Semantic | Actor relationships and trust levels | - |
| Device | Device registration with trust scores | INV64-65 |
| Autonomous | Behavioral pattern detection | INV34-37 |

**Infrastructure Layer**

| Pallet | Description | Invariants |
|--------|-------------|------------|
| Storage | Ephemeral data with retention policies | INV70-72 |
| Vault | Threshold-based secret sharing | INV66-68 |
| ZK | Zero-knowledge proof verification | INV73-75 |
| Octopus | Subnode cluster management | INV38-42, INV63 |
| Boomerang | Bidirectional path verification | INV30-33 |

## Protocol Constants

| Constant | Value | Invariant |
|----------|-------|-----------|
| Minimum Validators | 5 | INV46 |
| Max Stake Ratio | 33% | INV47 |
| Recovery Quorum | 80% | INV57 |
| Emergency Upgrade Quorum | 80% | INV60 |

## Contributing

Thank you for considering contributing to 7aychain. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Security

For security concerns, see [SECURITY.md](SECURITY.md) for our vulnerability disclosure policy.

## License

7aychain is licensed under the [Business Source License 1.1](LICENSE).

---

Built with [Substrate](https://substrate.io/) by [7ayLabs](https://7aylabs.com)
