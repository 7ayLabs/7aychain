//! Benchmarking setup for pallet-device
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
    fn register_device() {
        let caller: T::AccountId = whitelisted_caller();
        let device_type = DeviceType::Mobile;
        let public_key_hash = H256::repeat_byte(0xAA);
        let attestation_type = AttestationType::SelfSigned;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            device_type,
            public_key_hash,
            attestation_type,
        );
    }

    #[benchmark]
    fn activate_device() {
        let caller: T::AccountId = whitelisted_caller();
        let device_id = DeviceId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), device_id);
    }

    #[benchmark]
    fn suspend_device() {
        let caller: T::AccountId = whitelisted_caller();
        let device_id = DeviceId::new(1);
        let reason = H256::repeat_byte(0xBB);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), device_id, reason);
    }

    #[benchmark]
    fn revoke_device() {
        let caller: T::AccountId = whitelisted_caller();
        let device_id = DeviceId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), device_id);
    }

    #[benchmark]
    fn mark_compromised() {
        let device_id = DeviceId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Root, device_id);
    }

    #[benchmark]
    fn submit_attestation() {
        let caller: T::AccountId = whitelisted_caller();
        let device_id = DeviceId::new(1);
        let attestation_hash = H256::repeat_byte(0xCC);
        let attester: Option<seveny_primitives::types::ActorId> = None;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            device_id,
            attestation_hash,
            attester,
        );
    }

    #[benchmark]
    fn update_trust_score() {
        let device_id = DeviceId::new(1);
        let new_score: u8 = 85;

        #[extrinsic_call]
        _(RawOrigin::Root, device_id, new_score);
    }

    #[benchmark]
    fn record_activity() {
        let caller: T::AccountId = whitelisted_caller();
        let device_id = DeviceId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), device_id);
    }

    #[benchmark]
    fn reactivate_device() {
        let caller: T::AccountId = whitelisted_caller();
        let device_id = DeviceId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), device_id);
    }

    #[benchmark]
    fn record_heartbeat() {
        let caller: T::AccountId = whitelisted_caller();
        let device_id = DeviceId::new(1);
        let sequence: u64 = 1;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), device_id, sequence);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
