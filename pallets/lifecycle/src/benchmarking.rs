//! Benchmarking setup for pallet-lifecycle
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use seveny_primitives::types::ActorId;
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn register_actor() {
        let caller: T::AccountId = whitelisted_caller();
        let key_hash = H256::repeat_byte(0xAA);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), key_hash);
    }

    #[benchmark]
    fn activate_actor() {
        let actor_id = ActorId(H256::repeat_byte(0x01));

        #[extrinsic_call]
        _(RawOrigin::Root, actor_id);
    }

    #[benchmark]
    fn suspend_actor() {
        let actor_id = ActorId(H256::repeat_byte(0x01));

        #[extrinsic_call]
        _(RawOrigin::Root, actor_id);
    }

    #[benchmark]
    fn reactivate_actor() {
        let actor_id = ActorId(H256::repeat_byte(0x01));

        #[extrinsic_call]
        _(RawOrigin::Root, actor_id);
    }

    #[benchmark]
    fn initiate_destruction() {
        let caller: T::AccountId = whitelisted_caller();
        let reason = DestructionReason::OwnerRequest;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), reason);
    }

    #[benchmark]
    fn attest_destruction() {
        let caller: T::AccountId = whitelisted_caller();
        let target_actor = ActorId(H256::repeat_byte(0x02));
        let signature_hash = H256::repeat_byte(0xBB);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), target_actor, signature_hash);
    }

    #[benchmark]
    fn cancel_destruction() {
        let caller: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller));
    }

    #[benchmark]
    fn initiate_rotation() {
        let caller: T::AccountId = whitelisted_caller();
        let new_key_hash = H256::repeat_byte(0xCC);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), new_key_hash);
    }

    #[benchmark]
    fn complete_rotation() {
        let caller: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller));
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
