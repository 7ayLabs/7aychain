//! Protocol trait abstractions for the carrier vertical slice.
//!
//! Only traits actively used by the carrier flow (lifecycle, device,
//! presence, epoch, validator, carrier) are kept here. Speculative
//! trait abstractions were removed during the v0.9.7 scope reduction.

use sp_core::H256;

use crate::types::{ActorId, EpochId, ValidatorId};

/// Checks if an epoch is currently active.
/// Used by pallets that need cross-pallet epoch state without direct dependency.
pub trait EpochActiveChecker {
    fn is_epoch_active(epoch: crate::types::EpochId) -> bool;
}

/// Always returns true -- use in tests or pallets without epoch validation.
pub struct AlwaysActiveEpoch;
impl EpochActiveChecker for AlwaysActiveEpoch {
    fn is_epoch_active(_epoch: crate::types::EpochId) -> bool {
        true
    }
}

/// Checks if a validator is registered and active.
/// Used by pallets that need to verify validator identity without direct dependency.
pub trait ValidatorChecker {
    fn is_validator_registered(validator: crate::types::ValidatorId) -> bool;
}

/// Always returns true -- use in tests or pallets without validator validation.
pub struct AlwaysValidValidator;
impl ValidatorChecker for AlwaysValidValidator {
    fn is_validator_registered(_validator: crate::types::ValidatorId) -> bool {
        true
    }
}

/// Cross-pallet epoch state provider.
///
/// Allows pallets to query epoch state from the canonical epoch pallet
/// without maintaining shadow storage.
pub trait EpochProvider {
    fn is_epoch_active(epoch_id: EpochId) -> bool;
    fn current_epoch() -> EpochId;
}

/// Cross-pallet validator set provider.
///
/// Allows pallets to query validator status from the canonical validator
/// pallet without maintaining shadow storage.
pub trait ValidatorProvider {
    fn is_validator_active(validator_id: ValidatorId) -> bool;
}

/// Cross-pallet validator stake provider.
///
/// Allows pallets to query the current effective validator stake without
/// depending directly on the validator pallet's balance type.
pub trait ValidatorStakeProvider {
    fn validator_stake(validator_id: ValidatorId) -> u128;
}

/// Checks whether an actor lifecycle is currently active.
pub trait ActorActivityChecker {
    fn is_actor_active(actor_id: ActorId) -> bool;
}

/// Checks whether a device is active and controlled by the given actor.
pub trait DeviceEligibilityChecker {
    fn is_device_active_for_actor(actor_id: ActorId, device_id: u64) -> bool;
}

/// Checks whether Proof of Presence consensus has verified service eligibility
/// for the given actor and epoch.
pub trait PresenceVerifier {
    fn is_presence_verified(actor_id: ActorId, epoch_id: EpochId) -> bool;
}

/// Checks whether a carrier service node has verified position presence in
/// the given epoch.
pub trait ServiceNodePresenceVerifier<AccountId> {
    fn is_service_node_present(controller: &AccountId, epoch_id: EpochId) -> bool;
}

/// Reward handler for carrier service witnessing.
///
/// Called by `pallet-carrier` when finalizing service requests to pay
/// validators who attested carrier coverage. The `AccountId` is unused
/// for the validator-keyed variant; rewards are dispatched by `ValidatorId`.
pub trait CarrierRewardHandler<AccountId> {
    fn reward_witness(validator: &crate::types::ValidatorId, amount: u128);
}

/// No-op reward handler — use in tests or when rewards are disabled.
pub struct NoOpCarrierReward;
impl<AccountId> CarrierRewardHandler<AccountId> for NoOpCarrierReward {
    fn reward_witness(_validator: &crate::types::ValidatorId, _amount: u128) {}
}

/// Zero-stake provider for tests or runtimes without carrier service staking.
pub struct ZeroValidatorStake;
impl ValidatorStakeProvider for ZeroValidatorStake {
    fn validator_stake(_validator_id: ValidatorId) -> u128 {
        0
    }
}

/// Always-present service node verifier for tests.
pub struct AlwaysPresentServiceNode;
impl<AccountId> ServiceNodePresenceVerifier<AccountId> for AlwaysPresentServiceNode {
    fn is_service_node_present(_controller: &AccountId, _epoch_id: EpochId) -> bool {
        true
    }
}

/// Constant-time equality to prevent timing attacks.
pub trait ConstantTimeEq {
    fn ct_eq(&self, other: &Self) -> bool;
}

impl ConstantTimeEq for [u8; 32] {
    fn ct_eq(&self, other: &Self) -> bool {
        let mut acc: u8 = 0;
        for i in 0..32 {
            acc |= self[i] ^ other[i];
        }
        // Black-box hint prevents the compiler from short-circuiting
        // the loop or optimizing away the constant-time property.
        core::hint::black_box(acc) == 0
    }
}

impl ConstantTimeEq for H256 {
    fn ct_eq(&self, other: &Self) -> bool {
        self.0.ct_eq(&other.0)
    }
}

impl ConstantTimeEq for crate::crypto::PresenceCommitment {
    fn ct_eq(&self, other: &Self) -> bool {
        self.0.ct_eq(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ct_eq_bytes() {
        let a = [0u8; 32];
        let b = [0u8; 32];
        let c = [1u8; 32];

        assert!(a.ct_eq(&b));
        assert!(!a.ct_eq(&c));
    }

    #[test]
    fn ct_eq_h256() {
        let a = H256::zero();
        let b = H256::zero();
        let c = H256::repeat_byte(0xff);

        assert!(a.ct_eq(&b));
        assert!(!a.ct_eq(&c));
    }
}
