//! Benchmarking setup for pallet-octopus
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet::*;
use sp_core::H256;
use sp_runtime::Perbill;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn create_cluster() {
        let caller: T::AccountId = whitelisted_caller();
        let owner = H256::repeat_byte(0x01);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), owner);
    }

    #[benchmark]
    fn register_subnode() {
        let caller: T::AccountId = whitelisted_caller();
        let cluster_id = ClusterId::new(1);
        let operator = H256::repeat_byte(0x02);

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

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
