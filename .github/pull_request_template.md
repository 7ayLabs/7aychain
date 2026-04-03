## Problem

<!-- What weakness, bug, threat, or missing behavior is being changed? -->

## Goal

<!-- What must be true after this change? What must stay unchanged? -->

## Implementation

<!-- Runtime, pallet, node, infra, RPC, CI, or devnet surfaces changed. -->

## Test Plan

<!-- Exact tests and commands run for this change. -->

## Acceptance Evidence

<!-- Logs, events, storage, RPC results, screenshots, or command outcomes. -->

## Residual Risk

<!-- What remains unresolved, disabled, or deferred? -->

## Verification Checklist

- [ ] `cargo fmt --check`
- [ ] `cargo check -p seveny-runtime`
- [ ] Targeted crate tests for the touched subsystem
- [ ] Regression test for the exact bug, exploit path, or failure mode
- [ ] Devnet, multi-node, or real-device verification when behavior is operational
- [ ] Fail-closed behavior verified or documented

## Security / Privacy Notes

- Primary category:
- Secondary category:
- Unsafe defaults introduced or changed:
- New trust boundary introduced:
