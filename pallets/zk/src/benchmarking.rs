//! Benchmarking setup for pallet-zk
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use pallet::*;
use seveny_primitives::{
    crypto::{Nullifier, StateRoot},
    types::ActorId,
};
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn verify_share_proof() {
        let caller: T::AccountId = whitelisted_caller();
        let statement = ShareStatement {
            commitment_hash: H256::repeat_byte(0xCC),
        };
        let proof = BoundedVec::try_from(alloc::vec![0u8; 64]).expect("proof within bounds");

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), statement, proof);
    }

    #[benchmark]
    fn verify_presence_proof() {
        let caller: T::AccountId = whitelisted_caller();
        let statement = PresenceStatement {
            epoch_id: 1u64,
            state_root: StateRoot::EMPTY,
            nullifier: Nullifier(H256::repeat_byte(0xAA)),
        };
        let proof = BoundedVec::try_from(alloc::vec![0u8; 64]).expect("proof within bounds");

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), statement, proof);
    }

    #[benchmark]
    fn verify_access_proof() {
        let caller: T::AccountId = whitelisted_caller();
        let statement = AccessStatement {
            vault_id: 1u64,
            access_hash: H256::repeat_byte(0xEE),
        };
        let proof = BoundedVec::try_from(alloc::vec![0u8; 64]).expect("proof within bounds");

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), statement, proof);
    }

    #[benchmark]
    fn add_trusted_verifier() {
        let verifier = ActorId(H256::repeat_byte(0x01));

        #[extrinsic_call]
        _(RawOrigin::Root, verifier);
    }

    #[benchmark]
    fn remove_trusted_verifier() {
        let verifier = ActorId(H256::repeat_byte(0x01));

        #[extrinsic_call]
        _(RawOrigin::Root, verifier);
    }

    #[benchmark]
    fn consume_nullifier() {
        let caller: T::AccountId = whitelisted_caller();
        let nullifier = Nullifier(H256::repeat_byte(0xAA));

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), nullifier);
    }

    #[benchmark]
    fn register_circuit() {
        let circuit_id = H256::repeat_byte(0xDD);
        let proof_type = SnarkProofType::Groth16;
        let vk = BoundedVec::try_from(alloc::vec![0u8; 128]).expect("vk within bounds");

        #[extrinsic_call]
        _(RawOrigin::Root, circuit_id, proof_type, vk);
    }

    #[benchmark]
    fn verify_snark() {
        let caller: T::AccountId = whitelisted_caller();
        let circuit_id = H256::repeat_byte(0xDD);
        let proof = BoundedVec::try_from(alloc::vec![0u8; 64]).expect("proof within bounds");
        let inputs = BoundedVec::try_from(alloc::vec![[0u8; 32]; 1]).expect("inputs within bounds");

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), circuit_id, proof, inputs);
    }

    #[benchmark]
    fn transition_proof_system_mode() {
        let new_mode = migration::ProofSystemMode::SnarkOnly;

        #[extrinsic_call]
        _(RawOrigin::Root, new_mode);
    }

    #[benchmark]
    fn deregister_circuit() {
        let circuit_id = H256::repeat_byte(0xDD);

        #[extrinsic_call]
        _(RawOrigin::Root, circuit_id);
    }

    #[benchmark]
    fn emergency_revert_mode() {
        #[extrinsic_call]
        _(RawOrigin::Root);
    }

    #[benchmark]
    fn prune_old_proofs() {
        let older_than: frame_system::pallet_prelude::BlockNumberFor<T> = 10u32.into();
        let max_entries: u32 = 10;

        #[extrinsic_call]
        _(RawOrigin::Root, older_than, max_entries);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
