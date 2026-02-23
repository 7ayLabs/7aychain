<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="./.github/7aychain_dark_logo.svg?v=2">
    <img src="./.github/7aychain_white_logo.svg?v=2" alt="7aychain" width="500">
  </picture>
</p>

<p align="center">
  <a href="https://github.com/7ayLabs/7aychain/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/7ayLabs/7aychain/ci.yml?branch=main&style=for-the-badge" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-BUSL--1.1-blue.svg?style=for-the-badge" alt="License"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-stable-orange.svg?style=for-the-badge" alt="Rust"></a>
  <a href="https://github.com/paritytech/polkadot-sdk"><img src="https://img.shields.io/badge/substrate-polkadot--stable2503-blueviolet?style=for-the-badge" alt="Substrate"></a>
  <a href="https://github.com/7ayLabs/7aychain/releases/tag/v0.8.16"><img src="https://img.shields.io/badge/version-v0.8.16-green?style=for-the-badge" alt="Version"></a>
</p>

**7aychain** is a Layer 1 blockchain built to answer one question: _is this actor actually here?_ Validators form witness circles, measure network latency between peers, and triangulate positions — no GPS, no external oracles, no special hardware. Presence is verified through the protocol itself and finalized on-chain with quorum consensus.

The chain runs on the [7ay Proof of Presence Protocol](https://github.com/7ayLabs/7ay-presence), where every presence declaration goes through an epoch-bound lifecycle: declared, attested by witnesses, triangulated, and finalized by validators.

[Website](https://7aylabs.com) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [License](LICENSE)

Get running in three commands — clone, build, and start a local devnet. Then use the [Laud Networks CLI](#laud-networks-cli) to interact with every module on the chain.

```bash
git clone https://github.com/7ayLabs/7aychain.git && cd 7aychain
cargo build --release
./target/release/seveny-node --dev
```

---

## Building the Source

Building `7aychain` requires a Rust toolchain, Clang/LLVM, and protobuf.

### Prerequisites

Install Rust and the WASM target:

```shell
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown
```

Then install platform-specific dependencies:

<details>
<summary><b>macOS</b></summary>

```shell
brew install llvm cmake protobuf
```

</details>

<details>
<summary><b>Ubuntu / Debian</b></summary>

```shell
sudo apt-get update
sudo apt-get install -y clang libclang-dev protobuf-compiler pkg-config cmake build-essential
```

</details>

<details>
<summary><b>Fedora / RHEL</b></summary>

```shell
sudo dnf install clang clang-devel protobuf-compiler cmake pkg-config
```

</details>

<details>
<summary><b>Windows</b></summary>

Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) with the "C++ build tools" workload, then:

```powershell
choco install llvm cmake protoc
```

Or with Scoop:

```powershell
scoop install llvm cmake protobuf
```

</details>

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

## Hardware Requirements

| | Minimum | Recommended |
|---|---------|-------------|
| CPU | 2 cores | 4+ cores |
| RAM | 4 GB | 16 GB |
| Storage | 50 GB SSD | 100 GB NVMe SSD |
| Network | 8 Mbit/s | 25+ Mbit/s |

## Running 7aychain

### Development Node

Start a single-node dev chain with instant-seal and pre-funded accounts (Alice, Bob, Charlie, Dave, Eve, Ferdie):

```shell
./target/release/seveny-node --dev
```

With debug logging:

```shell
RUST_LOG=debug ./target/release/seveny-node --dev
```

### Single Node

Run a single validator on the local testnet:

```shell
./target/release/seveny-node \
  --alice \
  --validator \
  --chain local \
  --base-path /tmp/alice \
  --rpc-port 9944 \
  --rpc-cors all \
  --rpc-methods unsafe \
  --scanner-mode mock \
  --mock-devices 15
```

### Multi-Node Devnet

Run the full network with Alice natively and the remaining validators in Docker:

```shell
# 1. Start Alice natively (real or mock scanning)
devnet/scripts/run-native-alice.sh

# 2. Start Bob, Charlie, Dave, Eve, Ferdie in Docker
cd devnet
docker compose -f docker-compose.hybrid.yml up -d

# 3. Monitor the network
devnet/scripts/monitor.sh
```

Each node gets its own RPC port:

| Node | RPC Port | P2P Port |
|------|----------|----------|
| Alice (native) | 9944 | 30333 |
| Bob | 9945 | 30334 |
| Charlie | 9946 | 30335 |
| Dave | 9947 | 30336 |
| Eve | 9948 | 30337 |
| Ferdie | 9949 | 30338 |

Stop and reset:

```shell
docker compose -f docker-compose.hybrid.yml down       # stop Docker nodes
docker compose -f docker-compose.hybrid.yml down -v     # stop + clear chain state
```

### Configuration

| Flag | Description |
|------|-------------|
| `--dev` | Run in development mode with temporary storage |
| `--chain <spec>` | Chain specification (`dev`, `local`) |
| `--validator` | Enable validator mode |
| `--base-path <path>` | Database and keystore location |
| `--rpc-port <port>` | JSON-RPC port — serves both HTTP and WebSocket (default: 9944) |
| `--rpc-cors <origins>` | Allowed RPC origins (`all` for development) |
| `--rpc-methods <mode>` | RPC method set (`safe`, `unsafe`) |
| `--rpc-external` | Listen on all interfaces (0.0.0.0) |
| `--port <port>` | P2P network port (default: 30333) |
| `--name <name>` | Node display name |
| `--scanner-mode <mode>` | Device scanner mode (`latency`, `mock`) |
| `--mock-devices <n>` | Number of simulated devices in mock mode |
| `--scan-interval <secs>` | Seconds between device scans (default: 6) |
| `--scanner-pos-x/y/z <n>` | Scanner position coordinates |

## Docker

Quick start with a single instant-seal devnet node:

```shell
cd devnet
docker compose -f docker-compose.dev.yml up -d --build
```

Blocks are produced only when extrinsics are submitted.

| Port | Service |
|------|---------|
| `9944` | JSON-RPC (HTTP + WebSocket) |
| `30333` | P2P |

Stop and reset:

```shell
docker compose -f docker-compose.dev.yml down       # stop
docker compose -f docker-compose.dev.yml down -v     # stop + clear chain state
```

## Devnet Scripts

All scripts are in `devnet/scripts/`:

| Script | Description |
|--------|-------------|
| `dev.sh` | Start/stop single-node devnet (Docker or native) |
| `dev.sh native` | Run native binary without Docker |
| `dev.sh stop` | Stop the Docker container |
| `dev.sh reset` | Stop + clear chain state |
| `run-native-alice.sh` | Run Alice natively with real device scanning |
| `monitor.sh` | Real-time health monitor for multi-node devnet |

## Laud Networks CLI

Interactive testing suite for all protocol features.

### Quick Start

```shell
pip install substrate-interface
python3 devnet/scripts/laud-cli.py
```

Connects to `ws://127.0.0.1:9944` by default. Custom endpoint:

```shell
python3 devnet/scripts/laud-cli.py --url ws://host:port
```

### Commands

| Module | Operations |
|--------|------------|
| `presence` | declare, commit, reveal, vote, finalize, slash, quorum |
| `epoch` | schedule, start, close, finalize, register, update, force |
| `validator` | register, activate, deactivate, withdraw, stake, slash |
| `pbt` | position, claim, attest, verify, setup, test |
| `triangulation` | multilaterate, centroid, track |
| `dispute` | open, evidence, vote, resolve |
| `device` | register, scan, trust |
| `lifecycle` | register, destroy, status |
| `vault` | create, share, recover, register-file, request-unlock, authorize-unlock |
| `zk` | prove, verify |
| `governance` | propose, vote, delegate |
| `semantic` | link, trust, query |
| `boomerang` | send, verify |
| `autonomous` | detect, report |
| `octopus` | create, join, manage |
| `storage` | store, retrieve, expire |

Pre-loaded accounts: `alice`, `bob`, `charlie`, `dave`, `eve`, `ferdie`.

## Programmatic Access

### Polkadot.js Apps

Connect the hosted [Polkadot.js Apps](https://polkadot.js.org/apps/) to your local node:

```
Settings → Custom Endpoint → ws://127.0.0.1:9944
```

### JSON-RPC

```shell
curl -H "Content-Type: application/json" \
  -d '{"id":1, "jsonrpc":"2.0", "method":"system_health"}' \
  http://127.0.0.1:9944
```

### Subxt (Rust)

```rust
use subxt::{OnlineClient, PolkadotConfig};

let api = OnlineClient::<PolkadotConfig>::from_url("ws://127.0.0.1:9944").await?;
```

### @polkadot/api (JavaScript)

```javascript
const { ApiPromise, WsProvider } = require("@polkadot/api");

const api = await ApiPromise.create({ provider: new WsProvider("ws://127.0.0.1:9944") });
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Security

See [SECURITY.md](SECURITY.md) for our vulnerability disclosure policy.

## License

7aychain is licensed under the [Business Source License 1.1](LICENSE).

---

<p align="center">
  Built with <a href="https://substrate.io/">Substrate</a> by <a href="https://7aylabs.com">7ayLabs</a>
</p>
