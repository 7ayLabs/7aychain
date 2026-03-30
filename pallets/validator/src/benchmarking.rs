//! Benchmarking setup for pallet-validator
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet::*;
use seveny_primitives::types::ViolationType;
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn register_validator() {
        let caller: T::AccountId = whitelisted_caller();
        let stake: BalanceOf<T> = 1_000_000u32.into();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), stake);
    }

    #[benchmark]
    fn activate_validator() {
        let caller: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller));
    }

    #[benchmark]
    fn force_activate_validator() {
        let controller: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        _(RawOrigin::Root, controller);
    }

    #[benchmark]
    fn deactivate_validator() {
        let caller: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller));
    }

    #[benchmark]
    fn withdraw_stake() {
        let caller: T::AccountId = whitelisted_caller();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller));
    }

    #[benchmark]
    fn increase_stake() {
        let caller: T::AccountId = whitelisted_caller();
        let additional: BalanceOf<T> = 500_000u32.into();

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), additional);
    }

    #[benchmark]
    fn slash_validator() {
        let validator_id = H256::repeat_byte(0x01);
        let violation = ViolationType::Minor;

        #[extrinsic_call]
        _(RawOrigin::Root, validator_id, violation);
    }

    #[benchmark]
    fn apply_slash() {
        let slash_id: u64 = 1;

        #[extrinsic_call]
        _(RawOrigin::Root, slash_id);
    }

    #[benchmark]
    fn report_evidence() {
        let caller: T::AccountId = whitelisted_caller();
        let validator_id = H256::repeat_byte(0x01);
        let violation = ViolationType::Minor;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), validator_id, violation);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
