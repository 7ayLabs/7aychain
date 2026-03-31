//! Benchmarking setup for pallet-semantic
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
    fn create_relationship() {
        let caller: T::AccountId = whitelisted_caller();
        let to_actor = ActorId(H256::repeat_byte(0x02));
        let relationship_type = RelationshipType::Trust;
        let trust_level: u8 = 75;
        let expires_at: Option<frame_system::pallet_prelude::BlockNumberFor<T>> = None;
        let bidirectional = false;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            to_actor,
            relationship_type,
            trust_level,
            expires_at,
            bidirectional,
        );
    }

    #[benchmark]
    fn accept_relationship() {
        let caller: T::AccountId = whitelisted_caller();
        let relationship_id = RelationshipId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), relationship_id);
    }

    #[benchmark]
    fn revoke_relationship() {
        let caller: T::AccountId = whitelisted_caller();
        let relationship_id = RelationshipId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), relationship_id);
    }

    #[benchmark]
    fn update_trust_level() {
        let caller: T::AccountId = whitelisted_caller();
        let relationship_id = RelationshipId::new(1);
        let new_trust_level: u8 = 85;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), relationship_id, new_trust_level);
    }

    #[benchmark]
    fn request_discovery() {
        let caller: T::AccountId = whitelisted_caller();
        let criteria = DiscoveryCriteria {
            min_trust_level: 50,
            relationship_type: None,
            max_hops: 3,
            include_pending: false,
        };

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), criteria);
    }

    #[benchmark]
    fn update_profile() {
        let caller: T::AccountId = whitelisted_caller();
        let discovery_enabled = true;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), discovery_enabled);
    }

    #[benchmark]
    fn complete_discovery() {
        let request_id = DiscoveryRequestId::new(1);
        let results_count: u32 = 5;

        #[extrinsic_call]
        _(RawOrigin::Root, request_id, results_count);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
