//! Benchmarking setup for pallet-triangulation
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet::*;
use seveny_primitives::Position;
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn register_reporter() {
        let caller: T::AccountId = whitelisted_caller();
        let position = Position::new(40_000, -74_000, 0);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), position);
    }

    #[benchmark]
    fn deregister_reporter() {
        let caller: T::AccountId = whitelisted_caller();
        let reporter_id = ReporterId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), reporter_id);
    }

    #[benchmark]
    fn report_signal() {
        let caller: T::AccountId = whitelisted_caller();
        let reporter_id = ReporterId::new(1);
        let mac_hash = H256::repeat_byte(0xAA);
        let rssi: i8 = -65;
        let signal_type = SignalType::NetworkLatency;
        let frequency: u16 = 2400;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            reporter_id,
            mac_hash,
            rssi,
            signal_type,
            frequency,
        );
    }

    #[benchmark]
    fn update_reporter_position() {
        let caller: T::AccountId = whitelisted_caller();
        let reporter_id = ReporterId::new(1);
        let new_position = Position::new(41_000, -73_000, 0);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), reporter_id, new_position);
    }

    #[benchmark]
    fn submit_fraud_proof() {
        let caller: T::AccountId = whitelisted_caller();
        let submitter_id = ReporterId::new(1);
        let proof = FraudProof {
            accused_reporter: ReporterId::new(2),
            conflicting_readings: BoundedVec::default(),
            z_score_x100: 350,
            evidence_hash: H256::repeat_byte(0xEE),
        };

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), submitter_id, proof);
    }

    #[benchmark]
    fn resolve_fraud_case() {
        let reporter_id = ReporterId::new(1);
        let guilty = true;

        #[extrinsic_call]
        _(RawOrigin::Root, reporter_id, guilty);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
