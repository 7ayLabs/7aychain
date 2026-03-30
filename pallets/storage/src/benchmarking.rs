//! Benchmarking setup for pallet-storage
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet::*;
use seveny_primitives::types::{ActorId, EpochId};
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn store_data() {
        let caller: T::AccountId = whitelisted_caller();
        let epoch = EpochId::new(1);
        let key = DataKey::new(H256::repeat_byte(0x10));
        let data_hash = H256::repeat_byte(0xAA);
        let data_type = DataType::Temporary;
        let size_bytes: u32 = 1024;
        let retention = RetentionPolicy::EpochBound;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            epoch,
            key,
            data_hash,
            data_type,
            size_bytes,
            retention,
        );
    }

    #[benchmark]
    fn update_data() {
        let caller: T::AccountId = whitelisted_caller();
        let epoch = EpochId::new(1);
        let key = DataKey::new(H256::repeat_byte(0x10));
        let new_data_hash = H256::repeat_byte(0xBB);
        let new_size: u32 = 2048;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            epoch,
            key,
            new_data_hash,
            new_size,
        );
    }

    #[benchmark]
    fn delete_data() {
        let caller: T::AccountId = whitelisted_caller();
        let epoch = EpochId::new(1);
        let key = DataKey::new(H256::repeat_byte(0x10));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), epoch, key);
    }

    #[benchmark]
    fn set_quota() {
        let actor_id = ActorId(H256::repeat_byte(0x01));
        let max_entries: u32 = 100;
        let max_bytes: u64 = 10_000_000;

        #[extrinsic_call]
        _(RawOrigin::Root, actor_id, max_entries, max_bytes);
    }

    #[benchmark]
    fn finalize_epoch() {
        let epoch = EpochId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Root, epoch);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
