//! Benchmarking setup for pallet-presence
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet::*;
use seveny_primitives::{
    crypto::PresenceCommitment,
    types::{ActorId, EpochId, ValidatorId},
    Position,
};
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn declare_presence() {
        let caller: T::AccountId = whitelisted_caller();
        let epoch_id = EpochId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), epoch_id);
    }

    #[benchmark]
    fn declare_presence_with_commitment() {
        let caller: T::AccountId = whitelisted_caller();
        let epoch_id = EpochId::new(1);
        let commitment = PresenceCommitment(H256::repeat_byte(0xBB));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), epoch_id, commitment);
    }

    #[benchmark]
    fn vote_presence() {
        let caller: T::AccountId = whitelisted_caller();
        let actor_id = ActorId(H256::repeat_byte(0x01));
        let epoch_id = EpochId::new(1);
        let approve = true;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), actor_id, epoch_id, approve);
    }

    #[benchmark]
    fn finalize_presence() {
        let caller: T::AccountId = whitelisted_caller();
        let actor_id = ActorId(H256::repeat_byte(0x01));
        let epoch_id = EpochId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), actor_id, epoch_id);
    }

    #[benchmark]
    fn slash_presence() {
        let actor_id = ActorId(H256::repeat_byte(0x01));
        let epoch_id = EpochId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Root, actor_id, epoch_id);
    }

    #[benchmark]
    fn set_quorum_config() {
        let threshold: u32 = 3;
        let total: u32 = 5;

        #[extrinsic_call]
        _(RawOrigin::Root, threshold, total);
    }

    #[benchmark]
    fn reveal_commitment() {
        let caller: T::AccountId = whitelisted_caller();
        let epoch_id = EpochId::new(1);
        let secret: [u8; 32] = [0xCC; 32];
        let randomness: [u8; 32] = [0xDD; 32];

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), epoch_id, secret, randomness);
    }

    #[benchmark]
    fn claim_position() {
        let caller: T::AccountId = whitelisted_caller();
        let epoch_id = EpochId::new(1);
        let position = Position::new(40_000, -74_000, 0);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), epoch_id, position);
    }

    #[benchmark]
    fn submit_witness_attestation() {
        let caller: T::AccountId = whitelisted_caller();
        let target = ActorId(H256::repeat_byte(0x02));
        let epoch_id = EpochId::new(1);
        let latency_ms: u32 = 50;
        let direct_connection = true;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            target,
            epoch_id,
            latency_ms,
            direct_connection,
        );
    }

    #[benchmark]
    fn verify_position() {
        let caller: T::AccountId = whitelisted_caller();
        let target = ActorId(H256::repeat_byte(0x02));
        let epoch_id = EpochId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), target, epoch_id);
    }

    #[benchmark]
    fn set_validator_position() {
        let caller: T::AccountId = whitelisted_caller();
        let validator_id = ValidatorId(H256::repeat_byte(0x01));
        let position = Position::new(40_000, -74_000, 0);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), validator_id, position);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
