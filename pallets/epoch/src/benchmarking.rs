//! Benchmarking setup for pallet-epoch
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use seveny_primitives::types::{EpochId, EpochState};

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn schedule_epoch() {
        let start_block: frame_system::pallet_prelude::BlockNumberFor<T> = 100u32.into();
        let duration: frame_system::pallet_prelude::BlockNumberFor<T> = 1000u32.into();

        #[extrinsic_call]
        _(RawOrigin::Root, start_block, duration);
    }

    #[benchmark]
    fn start_epoch() {
        let epoch_id = EpochId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Root, epoch_id);
    }

    #[benchmark]
    fn close_epoch() {
        let epoch_id = EpochId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Root, epoch_id);
    }

    #[benchmark]
    fn finalize_epoch() {
        let epoch_id = EpochId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Root, epoch_id);
    }

    #[benchmark]
    fn register_participant() {
        let caller: T::AccountId = whitelisted_caller();
        let epoch_id = EpochId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), epoch_id);
    }

    #[benchmark]
    fn update_schedule() {
        let duration: frame_system::pallet_prelude::BlockNumberFor<T> = 2000u32.into();
        let grace_period: frame_system::pallet_prelude::BlockNumberFor<T> = 100u32.into();
        let auto_transition = true;

        #[extrinsic_call]
        _(RawOrigin::Root, duration, grace_period, auto_transition);
    }

    #[benchmark]
    fn force_transition() {
        let epoch_id = EpochId::new(1);
        let new_state = EpochState::Closed;

        #[extrinsic_call]
        _(RawOrigin::Root, epoch_id, new_state);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
