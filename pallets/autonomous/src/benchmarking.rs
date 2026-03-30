//! Benchmarking setup for pallet-autonomous
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet::*;
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn record_behavior() {
        let caller: T::AccountId = whitelisted_caller();
        let actor_id = H256::repeat_byte(0x01);
        let behavior_type = BehaviorType::PresencePattern;
        let data_hash = H256::repeat_byte(0xAA);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), actor_id, behavior_type, data_hash);
    }

    #[benchmark]
    fn register_pattern() {
        let behavior_type = BehaviorType::PresencePattern;
        let signature_hash = H256::repeat_byte(0xBB);
        let classification = PatternClassification::Normal;

        #[extrinsic_call]
        _(RawOrigin::Root, behavior_type, signature_hash, classification);
    }

    #[benchmark]
    fn classify_pattern() {
        let pattern_id = PatternId(1);
        let classification = PatternClassification::PotentiallyAutomated;
        let confidence_score: u8 = 80;

        #[extrinsic_call]
        _(RawOrigin::Root, pattern_id, classification, confidence_score);
    }

    #[benchmark]
    fn update_status() {
        let actor_id = H256::repeat_byte(0x01);
        let new_status = AutonomousStatus::Human;

        #[extrinsic_call]
        _(RawOrigin::Root, actor_id, new_status);
    }

    #[benchmark]
    fn flag_actor() {
        let actor_id = H256::repeat_byte(0x01);
        let reason = H256::repeat_byte(0xCC);

        #[extrinsic_call]
        _(RawOrigin::Root, actor_id, reason);
    }

    #[benchmark]
    fn match_behavior() {
        let behavior_id = BehaviorId::new(1);
        let actor_id = H256::repeat_byte(0x01);
        let pattern_id = PatternId(1);

        #[extrinsic_call]
        _(RawOrigin::Root, behavior_id, actor_id, pattern_id);
    }

    #[benchmark]
    fn create_profile() {
        let caller: T::AccountId = whitelisted_caller();
        let actor_id = H256::repeat_byte(0x01);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), actor_id);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
