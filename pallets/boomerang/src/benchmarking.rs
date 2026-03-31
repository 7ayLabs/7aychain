//! Benchmarking setup for pallet-boomerang
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet::*;
use seveny_primitives::types::ActorId;
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn initiate_path() {
        let caller: T::AccountId = whitelisted_caller();
        let target = ActorId(H256::repeat_byte(0x02));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), target);
    }

    #[benchmark]
    fn record_hop() {
        let caller: T::AccountId = whitelisted_caller();
        let path_id = PathId::new(1);
        let to_actor = ActorId(H256::repeat_byte(0x03));
        let signature_hash = H256::repeat_byte(0xBB);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), path_id, to_actor, signature_hash);
    }

    #[benchmark]
    fn extend_timeout() {
        let caller: T::AccountId = whitelisted_caller();
        let path_id = PathId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), path_id);
    }

    #[benchmark]
    fn fail_path() {
        let path_id = PathId::new(1);
        let reason = PathFailureReason::InvalidHop;

        #[extrinsic_call]
        _(RawOrigin::Root, path_id, reason);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
