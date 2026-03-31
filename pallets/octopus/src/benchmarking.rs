//! Benchmarking setup for pallet-octopus
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet::*;
use seveny_primitives::types::ActorId;
use sp_core::H256;
use sp_runtime::Perbill;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn create_cluster() {
        let caller: T::AccountId = whitelisted_caller();
        let owner = ActorId(H256::repeat_byte(0x01));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), owner);
    }

    #[benchmark]
    fn register_subnode() {
        let caller: T::AccountId = whitelisted_caller();
        let cluster_id = ClusterId::new(1);
        let operator = ActorId(H256::repeat_byte(0x02));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), cluster_id, operator);
    }

    #[benchmark]
    fn activate_subnode() {
        let caller: T::AccountId = whitelisted_caller();
        let subnode_id = SubnodeId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), subnode_id);
    }

    #[benchmark]
    fn start_deactivation() {
        let caller: T::AccountId = whitelisted_caller();
        let subnode_id = SubnodeId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), subnode_id);
    }

    #[benchmark]
    fn update_throughput() {
        let cluster_id = ClusterId::new(1);
        let throughput = Perbill::from_percent(75);

        #[extrinsic_call]
        _(RawOrigin::Root, cluster_id, throughput);
    }

    #[benchmark]
    fn evaluate_scaling() {
        let caller: T::AccountId = whitelisted_caller();
        let cluster_id = ClusterId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), cluster_id);
    }

    #[benchmark]
    fn update_subnode_throughput() {
        let caller: T::AccountId = whitelisted_caller();
        let subnode_id = SubnodeId::new(1);
        let throughput = Perbill::from_percent(50);
        let processed: u64 = 1000;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), subnode_id, throughput, processed);
    }

    #[benchmark]
    fn record_heartbeat() {
        let caller: T::AccountId = whitelisted_caller();
        let subnode_id = SubnodeId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), subnode_id);
    }

    #[benchmark]
    fn record_device_observation() {
        let caller: T::AccountId = whitelisted_caller();
        let subnode_id = SubnodeId::new(1);
        let device_count: u8 = 5;
        let commitment = H256::repeat_byte(0xDD);

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            subnode_id,
            device_count,
            commitment,
        );
    }

    #[benchmark]
    fn record_position_confirmation() {
        let caller: T::AccountId = whitelisted_caller();
        let subnode_id = SubnodeId::new(1);
        let position_x: i64 = 40_000;
        let position_y: i64 = -74_000;
        let position_z: i64 = 0;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            subnode_id,
            position_x,
            position_y,
            position_z,
        );
    }

    #[benchmark]
    fn heartbeat_with_device_proof() {
        let caller: T::AccountId = whitelisted_caller();
        let subnode_id = SubnodeId::new(1);
        let device_count: u8 = 3;
        let commitment = H256::repeat_byte(0xEE);

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            subnode_id,
            device_count,
            commitment,
        );
    }

    #[benchmark]
    fn set_fusion_weights() {
        let heartbeat_weight: u8 = 40;
        let device_weight: u8 = 30;
        let position_weight: u8 = 30;

        #[extrinsic_call]
        _(
            RawOrigin::Root,
            heartbeat_weight,
            device_weight,
            position_weight,
        );
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
