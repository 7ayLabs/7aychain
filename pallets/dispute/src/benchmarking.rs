//! Benchmarking setup for pallet-dispute
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet::*;
use seveny_primitives::types::{ValidatorId, ViolationType};
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn open_dispute() {
        let caller: T::AccountId = whitelisted_caller();
        let target = ValidatorId(H256::repeat_byte(0x01));
        let violation = ViolationType::Minor;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), target, violation);
    }

    #[benchmark]
    fn submit_evidence() {
        let caller: T::AccountId = whitelisted_caller();
        let dispute_id = DisputeId::new(1);
        let data_hash = H256::repeat_byte(0xBB);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), dispute_id, data_hash);
    }

    #[benchmark]
    fn resolve_dispute() {
        let dispute_id = DisputeId::new(1);
        let outcome = DisputeOutcome::ValidatorSlashed;

        #[extrinsic_call]
        _(RawOrigin::Root, dispute_id, outcome);
    }

    #[benchmark]
    fn reject_dispute() {
        let dispute_id = DisputeId::new(1);
        let reason = DisputeRejectionReason::InsufficientEvidence;

        #[extrinsic_call]
        _(RawOrigin::Root, dispute_id, reason);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
