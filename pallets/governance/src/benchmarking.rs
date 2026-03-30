//! Benchmarking setup for pallet-governance
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
    fn grant_capability() {
        let caller: T::AccountId = whitelisted_caller();
        let grantee = ActorId(H256::repeat_byte(0x01));
        let resource = ResourceId::from_bytes([0x20; 32]);
        let permissions = Permissions(0xFF);
        let expires_at: Option<frame_system::pallet_prelude::BlockNumberFor<T>> = None;
        let delegatable = false;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            grantee,
            resource,
            permissions,
            expires_at,
            delegatable,
        );
    }

    #[benchmark]
    fn revoke_capability() {
        let caller: T::AccountId = whitelisted_caller();
        let capability_id = CapabilityId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), capability_id);
    }

    #[benchmark]
    fn delegate_capability() {
        let caller: T::AccountId = whitelisted_caller();
        let capability_id = CapabilityId::new(1);
        let delegatee = ActorId(H256::repeat_byte(0x02));
        let permissions = Permissions(0x0F);
        let expires_at: Option<frame_system::pallet_prelude::BlockNumberFor<T>> = None;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            capability_id,
            delegatee,
            permissions,
            expires_at,
        );
    }

    #[benchmark]
    fn update_capability() {
        let caller: T::AccountId = whitelisted_caller();
        let capability_id = CapabilityId::new(1);
        let new_permissions = Permissions(0x0F);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), capability_id, new_permissions);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
