#![allow(clippy::disallowed_macros)]

use crate::{self as pallet_zk, *};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Zk: pallet_zk,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const MaxProofSize: u32 = 512;
    pub const MaxVerificationsPerBlock: u32 = 100;
}

impl pallet_zk::Config for Test {
    type WeightInfo = ();
    type MaxProofSize = MaxProofSize;
    type MaxVerificationsPerBlock = MaxVerificationsPerBlock;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("failed to build storage");
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn account_to_actor(account: u64) -> ActorId {
    use parity_scale_codec::Encode;
    let encoded = account.encode();
    let hash = sp_core::blake2_256(&encoded);
    ActorId::from_raw(hash)
}

fn create_share_witness() -> ShareWitness {
    ShareWitness {
        share_value: [1u8; 32],
        share_index: 0,
        randomness: [2u8; 32],
    }
}

fn create_presence_params() -> ([u8; 32], u64, u64, StateRoot) {
    let secret = [3u8; 32];
    let epoch_id = 1u64;
    let nonce = 42u64;
    let state_root = StateRoot::EMPTY;
    (secret, epoch_id, nonce, state_root)
}

fn create_access_params() -> (u64, ActorId, u32, H256) {
    let vault_id = 1u64;
    let actor_id = ActorId::from_raw([4u8; 32]);
    let ring_position = 0u32;
    let membership = H256([5u8; 32]);
    (vault_id, actor_id, ring_position, membership)
}

#[test]
fn verify_share_proof_success() {
    new_test_ext().execute_with(|| {
        let witness = create_share_witness();
        let (statement, proof) = Zk::generate_share_proof(&witness);
        let bounded_proof = BoundedVec::try_from(proof).expect("proof fits");

        assert_ok!(Zk::verify_share_proof(
            RuntimeOrigin::signed(1),
            statement.clone(),
            bounded_proof
        ));

        assert!(Zk::is_share_verified(&statement.commitment_hash));
        assert_eq!(Zk::total_verifications(), 1);
    });
}

#[test]
fn verify_presence_proof_success() {
    new_test_ext().execute_with(|| {
        let (secret, epoch_id, nonce, state_root) = create_presence_params();
        let (statement, proof) = Zk::generate_presence_proof(&secret, epoch_id, nonce, state_root);
        let bounded_proof = BoundedVec::try_from(proof).expect("proof fits");

        assert_ok!(Zk::verify_presence_proof(
            RuntimeOrigin::signed(1),
            statement.clone(),
            bounded_proof
        ));

        assert!(Zk::is_nullifier_used(&statement.nullifier));
        assert_eq!(Zk::total_verifications(), 1);
    });
}

#[test]
fn verify_access_proof_success() {
    new_test_ext().execute_with(|| {
        let (vault_id, actor_id, ring_position, membership) = create_access_params();
        let (statement, proof) =
            Zk::generate_access_proof(vault_id, &actor_id, ring_position, &membership);
        let bounded_proof = BoundedVec::try_from(proof).expect("proof fits");

        assert_ok!(Zk::verify_access_proof(
            RuntimeOrigin::signed(1),
            statement.clone(),
            bounded_proof
        ));

        assert!(Zk::is_access_verified(vault_id, &statement.access_hash));
        assert_eq!(Zk::total_verifications(), 1);
    });
}

#[test]
fn cannot_reuse_nullifier() {
    new_test_ext().execute_with(|| {
        let (secret, epoch_id, nonce, state_root) = create_presence_params();
        let (statement, proof) = Zk::generate_presence_proof(&secret, epoch_id, nonce, state_root);
        let bounded_proof = BoundedVec::try_from(proof.clone()).expect("proof fits");

        assert_ok!(Zk::verify_presence_proof(
            RuntimeOrigin::signed(1),
            statement.clone(),
            bounded_proof
        ));

        let bounded_proof2 = BoundedVec::try_from(proof).expect("proof fits");
        assert_noop!(
            Zk::verify_presence_proof(RuntimeOrigin::signed(1), statement, bounded_proof2),
            Error::<Test>::NullifierAlreadyUsed
        );
    });
}

#[test]
fn cannot_verify_same_statement_twice() {
    new_test_ext().execute_with(|| {
        let witness = create_share_witness();
        let (statement, proof) = Zk::generate_share_proof(&witness);
        let bounded_proof = BoundedVec::try_from(proof.clone()).expect("proof fits");

        assert_ok!(Zk::verify_share_proof(
            RuntimeOrigin::signed(1),
            statement.clone(),
            bounded_proof
        ));

        let bounded_proof2 = BoundedVec::try_from(proof).expect("proof fits");
        assert_noop!(
            Zk::verify_share_proof(RuntimeOrigin::signed(1), statement, bounded_proof2),
            Error::<Test>::StatementAlreadyVerified
        );
    });
}

#[test]
fn invalid_share_proof_rejected() {
    new_test_ext().execute_with(|| {
        let statement = ShareStatement {
            commitment_hash: H256([1u8; 32]),
        };
        let invalid_proof = vec![0u8; 65];
        let bounded_proof = BoundedVec::try_from(invalid_proof).expect("proof fits");

        assert_noop!(
            Zk::verify_share_proof(RuntimeOrigin::signed(1), statement, bounded_proof),
            Error::<Test>::ProofVerificationFailed
        );
    });
}

#[test]
fn invalid_presence_proof_rejected() {
    new_test_ext().execute_with(|| {
        let statement = PresenceStatement {
            epoch_id: 1,
            state_root: StateRoot::EMPTY,
            nullifier: Nullifier(H256([1u8; 32])),
        };
        let invalid_proof = vec![0u8; 80];
        let bounded_proof = BoundedVec::try_from(invalid_proof).expect("proof fits");

        assert_noop!(
            Zk::verify_presence_proof(RuntimeOrigin::signed(1), statement, bounded_proof),
            Error::<Test>::ProofVerificationFailed
        );
    });
}

#[test]
fn invalid_access_proof_rejected() {
    new_test_ext().execute_with(|| {
        let statement = AccessStatement {
            vault_id: 1,
            access_hash: H256([1u8; 32]),
        };
        let invalid_proof = vec![0u8; 68];
        let bounded_proof = BoundedVec::try_from(invalid_proof).expect("proof fits");

        assert_noop!(
            Zk::verify_access_proof(RuntimeOrigin::signed(1), statement, bounded_proof),
            Error::<Test>::ProofVerificationFailed
        );
    });
}

#[test]
fn add_trusted_verifier_success() {
    new_test_ext().execute_with(|| {
        let verifier = ActorId::from_raw([1u8; 32]);

        assert_ok!(Zk::add_trusted_verifier(RuntimeOrigin::root(), verifier));
        assert!(Zk::trusted_verifiers(verifier).unwrap_or(false));
    });
}

#[test]
fn remove_trusted_verifier_success() {
    new_test_ext().execute_with(|| {
        let verifier = ActorId::from_raw([1u8; 32]);

        assert_ok!(Zk::add_trusted_verifier(RuntimeOrigin::root(), verifier));
        assert_ok!(Zk::remove_trusted_verifier(RuntimeOrigin::root(), verifier));
        assert!(!Zk::trusted_verifiers(verifier).unwrap_or(false));
    });
}

#[test]
fn consume_nullifier_requires_trusted_verifier() {
    new_test_ext().execute_with(|| {
        let nullifier = Nullifier(H256([1u8; 32]));

        assert_noop!(
            Zk::consume_nullifier(RuntimeOrigin::signed(1), nullifier),
            Error::<Test>::NotTrustedVerifier
        );
    });
}

#[test]
fn consume_nullifier_success() {
    new_test_ext().execute_with(|| {
        let verifier_account = 1u64;
        let verifier = account_to_actor(verifier_account);

        assert_ok!(Zk::add_trusted_verifier(RuntimeOrigin::root(), verifier));

        let nullifier = Nullifier(H256([2u8; 32]));
        assert_ok!(Zk::consume_nullifier(
            RuntimeOrigin::signed(verifier_account),
            nullifier
        ));

        assert!(Zk::is_nullifier_used(&nullifier));
    });
}

#[test]
fn cannot_consume_nullifier_twice() {
    new_test_ext().execute_with(|| {
        let verifier_account = 1u64;
        let verifier = account_to_actor(verifier_account);

        assert_ok!(Zk::add_trusted_verifier(RuntimeOrigin::root(), verifier));

        let nullifier = Nullifier(H256([2u8; 32]));
        assert_ok!(Zk::consume_nullifier(
            RuntimeOrigin::signed(verifier_account),
            nullifier
        ));

        assert_noop!(
            Zk::consume_nullifier(RuntimeOrigin::signed(verifier_account), nullifier),
            Error::<Test>::NullifierAlreadyUsed
        );
    });
}

#[test]
fn verification_record_stored_correctly() {
    new_test_ext().execute_with(|| {
        let witness = create_share_witness();
        let (statement, proof) = Zk::generate_share_proof(&witness);
        let statement_hash = Zk::hash_statement(&statement.encode());
        let bounded_proof = BoundedVec::try_from(proof).expect("proof fits");

        assert_ok!(Zk::verify_share_proof(
            RuntimeOrigin::signed(1),
            statement,
            bounded_proof
        ));

        let record = Zk::get_verification_record(&statement_hash).expect("record exists");
        assert_eq!(record.proof_type, ProofType::Share);
        assert_eq!(record.status, VerificationStatus::Verified);
    });
}

#[test]
fn verifications_reset_each_block() {
    new_test_ext().execute_with(|| {
        let witness = create_share_witness();
        let (statement, proof) = Zk::generate_share_proof(&witness);
        let bounded_proof = BoundedVec::try_from(proof).expect("proof fits");

        assert_ok!(Zk::verify_share_proof(
            RuntimeOrigin::signed(1),
            statement,
            bounded_proof
        ));

        assert_eq!(Zk::verifications_this_block(), 1);

        System::set_block_number(2);
        Zk::on_initialize(2);

        assert_eq!(Zk::verifications_this_block(), 0);
    });
}

#[test]
fn multiple_proof_types() {
    new_test_ext().execute_with(|| {
        let share_witness = create_share_witness();
        let (share_statement, share_proof) = Zk::generate_share_proof(&share_witness);
        let share_bounded = BoundedVec::try_from(share_proof).expect("proof fits");

        assert_ok!(Zk::verify_share_proof(
            RuntimeOrigin::signed(1),
            share_statement,
            share_bounded
        ));

        let (secret, epoch_id, nonce, state_root) = create_presence_params();
        let (presence_statement, presence_proof) =
            Zk::generate_presence_proof(&secret, epoch_id, nonce, state_root);
        let presence_bounded = BoundedVec::try_from(presence_proof).expect("proof fits");

        assert_ok!(Zk::verify_presence_proof(
            RuntimeOrigin::signed(1),
            presence_statement,
            presence_bounded
        ));

        let (vault_id, actor_id, ring_position, membership) = create_access_params();
        let (access_statement, access_proof) =
            Zk::generate_access_proof(vault_id, &actor_id, ring_position, &membership);
        let access_bounded = BoundedVec::try_from(access_proof).expect("proof fits");

        assert_ok!(Zk::verify_access_proof(
            RuntimeOrigin::signed(1),
            access_statement,
            access_bounded
        ));

        assert_eq!(Zk::total_verifications(), 3);
    });
}

#[test]
fn proof_too_short_rejected() {
    new_test_ext().execute_with(|| {
        let statement = ShareStatement {
            commitment_hash: H256([1u8; 32]),
        };
        let short_proof = vec![0u8; 32];
        let bounded_proof = BoundedVec::try_from(short_proof).expect("proof fits");

        assert_noop!(
            Zk::verify_share_proof(RuntimeOrigin::signed(1), statement, bounded_proof),
            Error::<Test>::ProofVerificationFailed
        );
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        System::reset_events();

        let witness = create_share_witness();
        let (statement, proof) = Zk::generate_share_proof(&witness);
        let _statement_hash = Zk::hash_statement(&statement.encode());
        let bounded_proof = BoundedVec::try_from(proof).expect("proof fits");

        assert_ok!(Zk::verify_share_proof(
            RuntimeOrigin::signed(1),
            statement,
            bounded_proof
        ));

        let events = System::events();
        assert!(events
            .iter()
            .any(|e| matches!(&e.event, RuntimeEvent::Zk(Event::ShareProofVerified { .. }))));
    });
}

#[test]
fn genesis_initializes_correctly() {
    new_test_ext().execute_with(|| {
        assert_eq!(Zk::verification_count(), 0);
        assert_eq!(Zk::verifications_this_block(), 0);
    });
}

#[test]
fn different_epochs_different_nullifiers() {
    new_test_ext().execute_with(|| {
        let secret = [3u8; 32];
        let nonce = 42u64;
        let state_root = StateRoot::EMPTY;

        let (statement1, proof1) = Zk::generate_presence_proof(&secret, 1, nonce, state_root);
        let bounded1 = BoundedVec::try_from(proof1).expect("proof fits");

        assert_ok!(Zk::verify_presence_proof(
            RuntimeOrigin::signed(1),
            statement1.clone(),
            bounded1
        ));

        let (statement2, proof2) = Zk::generate_presence_proof(&secret, 2, nonce, state_root);
        let bounded2 = BoundedVec::try_from(proof2).expect("proof fits");

        assert_ok!(Zk::verify_presence_proof(
            RuntimeOrigin::signed(1),
            statement2.clone(),
            bounded2
        ));

        assert_ne!(statement1.nullifier, statement2.nullifier);
        assert!(Zk::is_nullifier_used(&statement1.nullifier));
        assert!(Zk::is_nullifier_used(&statement2.nullifier));
    });
}

#[test]
fn different_vaults_same_actor() {
    new_test_ext().execute_with(|| {
        let actor_id = ActorId::from_raw([4u8; 32]);
        let ring_position = 0u32;
        let membership = H256([5u8; 32]);

        let (statement1, proof1) =
            Zk::generate_access_proof(1, &actor_id, ring_position, &membership);
        let bounded1 = BoundedVec::try_from(proof1).expect("proof fits");

        assert_ok!(Zk::verify_access_proof(
            RuntimeOrigin::signed(1),
            statement1.clone(),
            bounded1
        ));

        let (statement2, proof2) =
            Zk::generate_access_proof(2, &actor_id, ring_position, &membership);
        let bounded2 = BoundedVec::try_from(proof2).expect("proof fits");

        assert_ok!(Zk::verify_access_proof(
            RuntimeOrigin::signed(1),
            statement2.clone(),
            bounded2
        ));

        assert!(Zk::is_access_verified(1, &statement1.access_hash));
        assert!(Zk::is_access_verified(2, &statement2.access_hash));
    });
}

#[test]
fn hash_statement_deterministic() {
    new_test_ext().execute_with(|| {
        let witness = create_share_witness();
        let (statement, _) = Zk::generate_share_proof(&witness);

        let hash1 = Zk::hash_statement(&statement.encode());
        let hash2 = Zk::hash_statement(&statement.encode());

        assert_eq!(hash1, hash2);
    });
}
