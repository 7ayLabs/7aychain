# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.8.16] - 2026-02-23

### Added

- **Vault Pallet**: `register_file` extrinsic for registering encrypted files with
  FEK shares against a vault ring (INV66-68)
- **Vault Pallet**: `request_unlock` extrinsic for initiating time-bounded threshold
  unlock ceremonies
- **Vault Pallet**: `authorize_unlock` extrinsic for ring members to submit FEK
  shares and authorize file decryption
- **Vault Pallet**: 6 new storage items (`VaultFiles`, `FilesByVault`,
  `UnlockRequests`, `UnlockAuthorizations`, `NextFileId`, `NextUnlockId`)
- **Vault Pallet**: New events (`FileRegistered`, `UnlockRequested`,
  `UnlockAuthorized`, `UnlockCompleted`)
- **Vault Pallet**: `weights.rs` with weight definitions for new extrinsics
- **Vault Pallet**: 516 lines of new tests covering file registration, unlock
  authorization, duplicate prevention, and expiry
- **Primitives**: `DOMAIN_VAULT_FEK`, `DOMAIN_VAULT_FILE`, and `DOMAIN_UNLOCK`
  domain separator constants
- **Primitives**: `key_fingerprint()` function for deterministic key identification
- **Storage Pallet**: `VaultFile` variant in `DataType` enum
- **Runtime**: `MaxFilesPerVault = 64` configuration constant
- **Runtime**: `UnlockPeriodBlocks = 300` configuration constant
- **LAUD CLI**: `laud_crypto.py` module with GF(2^8) Shamir secret sharing
  (Rijndael polynomial 0x11B)
- **LAUD CLI**: `laud_files.py` module with AES-256-GCM file encryption/decryption
- **LAUD CLI**: `laud_registry.py` data-driven command registry (1796 lines)
- **LAUD CLI**: Dual-mode system (Normal mode for end users, Dev mode for full
  pallet access)
- **LAUD CLI**: Secure document wizard and threshold unlock flow
- **LAUD CLI**: Box-drawn TUI with Unicode menus and epoch dashboard
- **LAUD CLI**: Per-domain and per-command contextual instructions

### Changed

- **Primitives**: Simplified `hash_with_domain()` to accept `&[u8]` directly
  (removed generic `AsRef<[u8]>`)
- **Primitives**: Simplified `shamir::split()` and `shamir::reconstruct()` (removed
  entropy parameter; entropy is now generated internally)
- **Primitives**: Added `nonce` parameter to `Nullifier::derive()` for enhanced
  uniqueness
- **Runtime**: Bumped `spec_version` from 100 to 101
- **README**: Lowered minimum hardware requirements to 2 cores / 4 GB RAM / 8 Mbit/s
- **LAUD CLI**: Replaced technical jargon with user-friendly language throughout
- **LAUD CLI**: Redesigned menus with box-drawing characters and epoch dashboard
- **LAUD CLI**: Robust Aura slot timing with retry logic on pool errors

### Fixed

- Resolved clippy `derivable_impls` warnings across autonomous, boomerang, device,
  governance, lifecycle, octopus, semantic, triangulation, vault, and zk pallets
- Replaced `expect()` calls with proper error handling (`ok_or`,
  `unwrap_or_default`) across all affected pallets
- Fixed LAUD CLI menu display consuming first user input
- Fixed LAUD CLI 7s Aura slot delay restoration in submission logic
- Added 1s delay between LAUD CLI submissions to prevent Aura slot panics
- Fixed 12 audit findings in LAUD CLI command registry

### Security

- Removed tracked Python bytecache file from repository
  (`__pycache__/laud-cli.cpython-314.pyc`)
- Added `__pycache__/`, `*.pyc`, and `*_test_key*` patterns to `.gitignore`
- AES-256-GCM authenticated encryption for vault file operations (CLI-side)

### Breaking

- `hash_with_domain()` signature changed: callers passing owned types must add
  `.as_ref()`
- `shamir::split()` no longer accepts an entropy parameter
- `Nullifier::derive()` now requires a `nonce` argument (use `0u64` for
  backward-compatible behavior)
- `spec_version` bumped to 101: runtime upgrade required for existing chains

[v0.8.16]: https://github.com/7ayLabs/7aychain/releases/tag/v0.8.16
