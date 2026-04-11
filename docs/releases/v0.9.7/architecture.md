# v0.9.7 Architecture

## Summary

`7AYchain` should evolve into the decentralized control plane of a mobile
carrier, with Proof of Presence as the consensus primitive that authorizes
subscriber service.

The chain should not attempt to directly transport radio signaling or audio
media. Instead, it should decide the trust-critical telecom facts that the
off-chain telecom stack must obey.

## Control Plane Responsibilities

The chain is responsible for:

- actor identity lifecycle
- device trust lifecycle
- number ownership and assignment
- SIM/eSIM entitlement binding
- regional presence verification
- attach authorization policy
- portability and recovery
- disputes and fraud penalties

## Native Carrier Flow

1. An actor is registered and active.
2. A device is registered and active.
3. A number is assigned to that actor.
4. A SIM/eSIM entitlement is bound to the subscriber profile.
5. The actor satisfies local Proof of Presence in the target service region.
6. Carrier witnesses finalize service activation.
7. The bridge derives an attach lease from the chain.
8. The mobile core admits the handset.
9. Native voice/data/SMS service is provided by the telecom stack.

## Proof Of Presence Adaptation

Generic presence is not sufficient for native carrier service. For `v0.9.7`,
the proof must become locality-aware:

- the claimant must be in the serving region
- the witnesses must be local carrier witnesses or equivalent local validators
- the resulting authorization must be time-bounded
- the authorization must be bound to actor, device, region, and number

This produces a service lease rather than a generic presence record.

## Minimal Telecom Bridge Contract

The bridge should consume:

- actor status
- device status
- number binding status
- SIM/eSIM commitment status
- latest valid presence lease
- suspension or dispute status

The bridge should output:

- attach allowed / denied
- service region
- entitlement duration
- portability or recovery hold state

## Security Constraints

`v0.9.7` must treat the following as consensus-critical:

- SIM swap protection
- number hijack prevention
- duplicate number binding prevention
- stale-presence rejection
- locality spoofing resistance
- malicious witness slashing
- recovery path abuse prevention
