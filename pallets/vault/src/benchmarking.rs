//! Benchmarking setup for pallet-vault
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
    fn create_vault() {
        let caller: T::AccountId = whitelisted_caller();
        let owner = ActorId(H256::repeat_byte(0x01));
        let threshold: u32 = 2;
        let ring_size: u32 = 3;
        let secret_hash = H256::repeat_byte(0xAA);

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            owner,
            threshold,
            ring_size,
            secret_hash,
        );
    }

    #[benchmark]
    fn add_member() {
        let caller: T::AccountId = whitelisted_caller();
        let vault_id = VaultId::new(1);
        let member = ActorId(H256::repeat_byte(0x02));
        let role = MemberRole::Participant;

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), vault_id, member, role);
    }

    #[benchmark]
    fn activate_vault() {
        let caller: T::AccountId = whitelisted_caller();
        let vault_id = VaultId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), vault_id);
    }

    #[benchmark]
    fn commit_share() {
        let caller: T::AccountId = whitelisted_caller();
        let vault_id = VaultId::new(1);
        let commitment = H256::repeat_byte(0xCC);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), vault_id, commitment);
    }

    #[benchmark]
    fn initiate_recovery() {
        let caller: T::AccountId = whitelisted_caller();
        let vault_id = VaultId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), vault_id);
    }

    #[benchmark]
    fn reveal_share() {
        let caller: T::AccountId = whitelisted_caller();
        let share_id = ShareId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), share_id);
    }

    #[benchmark]
    fn lock_vault() {
        let caller: T::AccountId = whitelisted_caller();
        let vault_id = VaultId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), vault_id);
    }

    #[benchmark]
    fn dissolve_vault() {
        let vault_id = VaultId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Root, vault_id);
    }

    #[benchmark]
    fn register_file() {
        let caller: T::AccountId = whitelisted_caller();
        let vault_id = VaultId::new(1);
        let enc_hash = H256::repeat_byte(0xEE);
        let plaintext_hash = H256::repeat_byte(0xFF);
        let key_fingerprint = H256::repeat_byte(0xAA);
        let size_bytes: u64 = 1024;

        #[extrinsic_call]
        _(
            RawOrigin::Signed(caller),
            vault_id,
            enc_hash,
            plaintext_hash,
            key_fingerprint,
            size_bytes,
        );
    }

    #[benchmark]
    fn request_unlock() {
        let caller: T::AccountId = whitelisted_caller();
        let vault_id = VaultId::new(1);
        let file_enc_hash = H256::repeat_byte(0xFF);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), vault_id, file_enc_hash);
    }

    #[benchmark]
    fn authorize_unlock() {
        let caller: T::AccountId = whitelisted_caller();
        let request_id = UnlockRequestId::new(1);

        #[extrinsic_call]
        _(RawOrigin::Signed(caller), request_id);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
