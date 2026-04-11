# v0.9.7 Native Carrier Devnet

`v0.9.7` is the release target that turns `7AYchain` from a Proof-of-Presence
DePIN chain into the control plane of a decentralized mobile carrier devnet.

The final objective of this release is not a VoIP-only demo. It is a devnet in
which `7AYchain` authorizes native subscriber service, number binding, and
regional service activation so that an iPhone or another handset can attach to
real carrier infrastructure identified as `7AY`.

## Release Branch

Canonical release branch:

- `feat/v0.9.7-native-carrier-devnet`

This matches the repository's release branch pattern:

- `feat/v0.9.0-foundation-complete`
- `feat/v0.9.1-crypto-hardening`
- `feat/v0.9.3-real-device-devnet`
- `feat/v0.9.5-security-planning-mode`

## Branch Structure

Recommended sub-branches for `v0.9.7`:

- `feat/v0.9.7-carrier-core`
- `feat/v0.9.7-numbering-and-subscriber-binding`
- `feat/v0.9.7-esim-subscriber-profiles`
- `feat/v0.9.7-core-network-bridge`
- `feat/v0.9.7-ran-devnet`
- `feat/v0.9.7-ios-native-attach`
- `feat/v0.9.7-carrier-rpc-indexer`
- `security/v0.9.7-subscriber-hardening`
- `security/v0.9.7-anti-sim-swap`
- `test/v0.9.7-native-call-validation`
- `docs/v0.9.7-release-spec`

Use `feat/` for new capability delivery, `security/` for fraud and abuse
controls, `test/` for validation work, and `docs/` for release docs.

## Commit Syntax

Use conventional commits, matching the existing history:

- `feat(carrier): add subscriber-core state machine for number activation`
- `feat(devnet): add 7AY core bridge for attach authorization`
- `feat(rpc): expose carrier subscriber and number status endpoints`
- `fix(carrier): reject attach authorization without verified presence lease`
- `security(carrier): require fresh PoP quorum for SIM rotation`
- `test(devnet): add native attach and call validation harness`
- `docs(release): add v0.9.7 native carrier devnet release spec`
- `build: bump version to v0.9.7 and spec_version to <next>`

Prefer narrow scopes:

- `carrier`
- `subscriber`
- `numbering`
- `esim`
- `devnet`
- `rpc`
- `runtime`
- `ios`
- `bridge`

## Objective

`7AYchain` must remain the consensus and trust layer.

It does **not** replace LTE/5G radio signaling directly. Instead, it becomes the
authoritative control plane that decides:

- who is a valid subscriber
- which device is authorized
- which number is assigned
- which region the subscriber may activate in
- whether local Proof of Presence consensus has been satisfied
- whether a SIM/eSIM rotation, portability request, or recovery action is valid

The native telecom stack then consumes this state to provide actual network
service seen by the handset.

## Architecture Target

`v0.9.7` should converge on this stack:

- `7AYchain`
  Proof of Presence consensus, subscriber rights, number ownership, disputes,
  portability, service policies
- `carrier-core pallet`
  Numbering, subscriber binding, SIM/eSIM entitlement, service activation,
  attach leases, recovery
- `carrier bridge`
  Reads chain state and translates it into attach authorization for the mobile
  core
- `mobile core`
  Real subscriber attach, session authorization, policy control
- `RAN / local coverage nodes`
  Real radio access or controlled test infrastructure for native signal
- `IMS / voice path`
  Native call handling once attach is valid

## Protocol Rule

Carrier-specific service must remain subordinate to Proof of Presence.

The canonical attach rule for `v0.9.7` is:

`Active Actor + Active Device + Bound Number + Valid SIM/eSIM Entitlement + Verified Local Proof of Presence + Local Witness Quorum = Attach Authorized`

The canonical number-activation rule is:

`Identity Eligibility + Number Assignment + Device Binding + Presence Verification + Witness Finality = Active Subscriber Number`

## Required On-Chain Additions

`v0.9.7` should introduce a carrier-specific protocol module. The preferred
shape is either a new `pallet-carrier` or a narrowly equivalent pallet with the
following responsibilities:

- number reservation and assignment
- subscriber-to-number binding
- SIM/eSIM profile commitment binding
- presence-gated service activation
- attach authorization leases
- number portability
- device or number recovery
- suspension and dispute hooks

Suggested core state:

- `NumberId`
- `SubscriberProfile`
- `SubscriberBinding`
- `SimProfileCommitment`
- `ActivationRequest`
- `AttachLease`
- `PortRequest`
- `RecoveryRequest`

Suggested state machine:

- `Requested`
- `PresenceClaimed`
- `Witnessed`
- `Validated`
- `Activated`
- `AttachAuthorized`
- `Porting`
- `Suspended`
- `RecoveryPending`
- `Recovered`
- `Revoked`

## Required Off-Chain Additions

`v0.9.7` is not complete without the telecom bridge layer:

- carrier bridge service
- subscriber/indexer API
- SIM/eSIM entitlement tooling
- devnet mobile core integration
- controlled RAN or local signal test environment
- native call validation path

## Acceptance Criteria

`v0.9.7` should be considered complete only when all of the following are true:

- a subscriber can be registered on-chain with device and number binding
- service activation depends on Proof of Presence consensus, not an admin-only
  override
- a telecom bridge can derive attach authorization from on-chain state
- the devnet can expose real `7AY` service state to a handset through native
  carrier infrastructure
- an iPhone or other supported device can observe native service associated with
  a `7AY` subscriber number
- the devnet includes a reproducible validation flow for native attach and call
  testing

## Definition Of Done

`v0.9.7` is done when the native-carrier devnet proves that `7AYchain` is not
just tracking identities or presence records, but actually controlling the
subscriber truth required for real-number service on handset-native carrier
infrastructure.
