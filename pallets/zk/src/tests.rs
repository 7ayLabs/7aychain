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
    pub const MaxCircuits: u32 = 256;
}

impl pallet_zk::Config for Test {
    type WeightInfo = ();
    type Verifier = crate::StubVerifier;
    type MaxProofSize = MaxProofSize;
    type MaxVerificationsPerBlock = MaxVerificationsPerBlock;
    type MaxCircuits = MaxCircuits;
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

fn create_presence_params() -> ([u8; 32], u64, StateRoot) {
    let secret = [3u8; 32];
    let epoch_id = 1u64;
    let state_root = StateRoot::EMPTY;
    (secret, epoch_id, state_root)
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
        let (secret, epoch_id, state_root) = create_presence_params();
        let (statement, proof) = Zk::generate_presence_proof(&secret, epoch_id, state_root);
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
        let (secret, epoch_id, state_root) = create_presence_params();
        let (statement, proof) = Zk::generate_presence_proof(&secret, epoch_id, state_root);
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
        assert!(Zk::trusted_verifiers(verifier));
    });
}

#[test]
fn remove_trusted_verifier_success() {
    new_test_ext().execute_with(|| {
        let verifier = ActorId::from_raw([1u8; 32]);

        assert_ok!(Zk::add_trusted_verifier(RuntimeOrigin::root(), verifier));
        assert_ok!(Zk::remove_trusted_verifier(RuntimeOrigin::root(), verifier));
        assert!(!Zk::trusted_verifiers(verifier));
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

        let (secret, epoch_id, state_root) = create_presence_params();
        let (presence_statement, presence_proof) =
            Zk::generate_presence_proof(&secret, epoch_id, state_root);
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
        let state_root = StateRoot::EMPTY;

        let (statement1, proof1) = Zk::generate_presence_proof(&secret, 1, state_root);
        let bounded1 = BoundedVec::try_from(proof1).expect("proof fits");

        assert_ok!(Zk::verify_presence_proof(
            RuntimeOrigin::signed(1),
            statement1.clone(),
            bounded1
        ));

        let (statement2, proof2) = Zk::generate_presence_proof(&secret, 2, state_root);
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

// ===================================================================
// INV73: Share proofs must verify against commitments
// ===================================================================

#[test]
fn inv73_share_proof_commitment_binding() {
    new_test_ext().execute_with(|| {
        let witness = create_share_witness();
        let (statement, proof) = Zk::generate_share_proof(&witness);

        // Valid proof passes
        let bounded = BoundedVec::try_from(proof).expect("fits");
        assert_ok!(Zk::verify_share_proof(
            RuntimeOrigin::signed(1),
            statement,
            bounded
        ));
    });
}

#[test]
fn inv73_share_proof_wrong_commitment_rejected() {
    new_test_ext().execute_with(|| {
        let witness = create_share_witness();
        let (_, proof) = Zk::generate_share_proof(&witness);

        // Tamper with the commitment hash
        let bad_statement = ShareStatement {
            commitment_hash: H256([0xAA; 32]),
        };

        let bounded = BoundedVec::try_from(proof).expect("fits");
        assert_noop!(
            Zk::verify_share_proof(RuntimeOrigin::signed(1), bad_statement, bounded),
            Error::<Test>::ProofVerificationFailed
        );
    });
}

#[test]
fn inv73_share_proof_different_witnesses_different_commitments() {
    new_test_ext().execute_with(|| {
        let witness1 = ShareWitness {
            share_value: [1u8; 32],
            share_index: 0,
            randomness: [2u8; 32],
        };
        let witness2 = ShareWitness {
            share_value: [10u8; 32],
            share_index: 1,
            randomness: [20u8; 32],
        };

        let (stmt1, _) = Zk::generate_share_proof(&witness1);
        let (stmt2, _) = Zk::generate_share_proof(&witness2);

        assert_ne!(stmt1.commitment_hash, stmt2.commitment_hash);
    });
}

// ===================================================================
// INV74: Presence proofs must verify statement validity
// ===================================================================

#[test]
fn inv74_presence_proof_no_raw_secret_in_extrinsic() {
    new_test_ext().execute_with(|| {
        let secret = [42u8; 32];
        let epoch_id = 1u64;
        let state_root = StateRoot::EMPTY;

        let (_, proof) = Zk::generate_presence_proof(&secret, epoch_id, state_root);

        // The raw secret must NOT appear anywhere in the proof bytes
        assert!(
            !proof.windows(32).any(|w| w == secret),
            "INV74 violation: raw secret found in proof data"
        );
    });
}

#[test]
fn inv74_presence_proof_zero_commitment_rejected() {
    new_test_ext().execute_with(|| {
        let secret = [3u8; 32];
        let epoch_id = 1u64;
        let nullifier = seveny_primitives::crypto::Nullifier::derive(&secret, epoch_id);

        let statement = PresenceStatement {
            epoch_id,
            state_root: StateRoot::EMPTY,
            nullifier,
        };

        // Craft a proof with zero commitment (should be rejected)
        let mut bad_proof = vec![0u8; 80];
        // Leave commitment as all zeros
        // Set correct nullifier binding in bytes 32..64
        let mut null_input = Vec::with_capacity(40);
        null_input.extend_from_slice(nullifier.0.as_bytes());
        null_input.extend_from_slice(&epoch_id.to_le_bytes());
        let binding = sp_core::blake2_256(&null_input);
        bad_proof[32..64].copy_from_slice(&binding);

        let bounded = BoundedVec::try_from(bad_proof).expect("fits");
        assert_noop!(
            Zk::verify_presence_proof(RuntimeOrigin::signed(1), statement, bounded),
            Error::<Test>::ProofVerificationFailed
        );
    });
}

#[test]
fn inv74_same_secret_different_epochs_different_nullifiers() {
    new_test_ext().execute_with(|| {
        let secret = [7u8; 32];
        let state_root = StateRoot::EMPTY;

        let (stmt1, _) = Zk::generate_presence_proof(&secret, 1, state_root);
        let (stmt2, _) = Zk::generate_presence_proof(&secret, 2, state_root);
        let (stmt3, _) = Zk::generate_presence_proof(&secret, 3, state_root);

        // All nullifiers must be unique
        assert_ne!(stmt1.nullifier, stmt2.nullifier);
        assert_ne!(stmt2.nullifier, stmt3.nullifier);
        assert_ne!(stmt1.nullifier, stmt3.nullifier);
    });
}

#[test]
fn inv74_different_secrets_same_epoch_different_nullifiers() {
    new_test_ext().execute_with(|| {
        let state_root = StateRoot::EMPTY;

        let (stmt1, _) = Zk::generate_presence_proof(&[1u8; 32], 1, state_root);
        let (stmt2, _) = Zk::generate_presence_proof(&[2u8; 32], 1, state_root);

        assert_ne!(stmt1.nullifier, stmt2.nullifier);
    });
}

// ===================================================================
// INV75: Access proofs must verify authorization
// ===================================================================

#[test]
fn inv75_access_proof_vault_binding() {
    new_test_ext().execute_with(|| {
        let (vault_id, actor_id, ring_position, membership) = create_access_params();
        let (statement, proof) =
            Zk::generate_access_proof(vault_id, &actor_id, ring_position, &membership);
        let bounded = BoundedVec::try_from(proof).expect("fits");

        assert_ok!(Zk::verify_access_proof(
            RuntimeOrigin::signed(1),
            statement.clone(),
            bounded
        ));

        assert!(Zk::is_access_verified(vault_id, &statement.access_hash));
    });
}

#[test]
fn inv75_access_proof_wrong_membership_rejected() {
    new_test_ext().execute_with(|| {
        let actor_id = ActorId::from_raw([4u8; 32]);
        let ring_position = 0u32;
        let membership1 = H256([5u8; 32]);
        let membership2 = H256([99u8; 32]);

        // Generate proof with membership1
        let (_, proof) = Zk::generate_access_proof(1, &actor_id, ring_position, &membership1);

        // Try to verify against statement computed with membership2
        let (bad_statement, _) =
            Zk::generate_access_proof(1, &actor_id, ring_position, &membership2);

        let bounded = BoundedVec::try_from(proof).expect("fits");
        assert_noop!(
            Zk::verify_access_proof(RuntimeOrigin::signed(1), bad_statement, bounded),
            Error::<Test>::ProofVerificationFailed
        );
    });
}

#[test]
fn inv75_access_proof_wrong_actor_rejected() {
    new_test_ext().execute_with(|| {
        let actor1 = ActorId::from_raw([4u8; 32]);
        let actor2 = ActorId::from_raw([99u8; 32]);
        let ring_position = 0u32;
        let membership = H256([5u8; 32]);

        // Generate proof for actor1
        let (_, proof) = Zk::generate_access_proof(1, &actor1, ring_position, &membership);

        // Try to verify with actor2's statement
        let (bad_statement, _) = Zk::generate_access_proof(1, &actor2, ring_position, &membership);

        let bounded = BoundedVec::try_from(proof).expect("fits");
        assert_noop!(
            Zk::verify_access_proof(RuntimeOrigin::signed(1), bad_statement, bounded),
            Error::<Test>::ProofVerificationFailed
        );
    });
}

// ===================================================================
// Circuit registration and SNARK verification
// ===================================================================

#[test]
fn register_circuit_success() {
    new_test_ext().execute_with(|| {
        let circuit_id = H256([10u8; 32]);
        let vk = BoundedVec::try_from(vec![1u8; 256]).expect("fits");

        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            circuit_id,
            SnarkProofType::Groth16,
            vk
        ));

        assert!(Zk::circuit_registry(circuit_id).is_some());
        assert!(Zk::verification_keys(circuit_id).is_some());
    });
}

#[test]
fn register_circuit_requires_root() {
    new_test_ext().execute_with(|| {
        let circuit_id = H256([10u8; 32]);
        let vk = BoundedVec::try_from(vec![1u8; 256]).expect("fits");

        assert_noop!(
            Zk::register_circuit(
                RuntimeOrigin::signed(1),
                circuit_id,
                SnarkProofType::Groth16,
                vk
            ),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn register_circuit_duplicate_rejected() {
    new_test_ext().execute_with(|| {
        let circuit_id = H256([10u8; 32]);
        let vk = BoundedVec::try_from(vec![1u8; 256]).expect("fits");

        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            circuit_id,
            SnarkProofType::Groth16,
            vk.clone()
        ));

        assert_noop!(
            Zk::register_circuit(
                RuntimeOrigin::root(),
                circuit_id,
                SnarkProofType::Groth16,
                vk
            ),
            Error::<Test>::CircuitAlreadyRegistered
        );
    });
}

#[test]
fn verify_snark_requires_trusted_verifier() {
    new_test_ext().execute_with(|| {
        let circuit_id = H256([10u8; 32]);
        let vk = BoundedVec::try_from(vec![1u8; 256]).expect("fits");

        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            circuit_id,
            SnarkProofType::Groth16,
            vk
        ));

        let proof = BoundedVec::try_from(vec![1u8; 256]).expect("fits");
        let inputs = BoundedVec::try_from(vec![[1u8; 32]]).expect("fits");

        assert_noop!(
            Zk::verify_snark(RuntimeOrigin::signed(1), circuit_id, proof, inputs),
            Error::<Test>::NotTrustedVerifier
        );
    });
}

#[test]
fn verify_snark_circuit_not_found() {
    new_test_ext().execute_with(|| {
        let verifier_account = 1u64;
        let verifier = account_to_actor(verifier_account);
        assert_ok!(Zk::add_trusted_verifier(RuntimeOrigin::root(), verifier));

        let proof = BoundedVec::try_from(vec![1u8; 256]).expect("fits");
        let inputs = BoundedVec::try_from(vec![[1u8; 32]]).expect("fits");

        assert_noop!(
            Zk::verify_snark(
                RuntimeOrigin::signed(verifier_account),
                H256([99u8; 32]),
                proof,
                inputs
            ),
            Error::<Test>::CircuitNotFound
        );
    });
}

#[test]
fn verify_snark_success_with_stub() {
    new_test_ext().execute_with(|| {
        let verifier_account = 1u64;
        let verifier = account_to_actor(verifier_account);
        assert_ok!(Zk::add_trusted_verifier(RuntimeOrigin::root(), verifier));

        let circuit_id = H256([10u8; 32]);
        let vk = BoundedVec::try_from(vec![1u8; 256]).expect("fits");
        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            circuit_id,
            SnarkProofType::Groth16,
            vk
        ));

        // StubVerifier::verify_snark needs len >= 192 and non-empty inputs
        let proof = BoundedVec::try_from(vec![1u8; 256]).expect("fits");
        let inputs = BoundedVec::try_from(vec![[1u8; 32]]).expect("fits");

        assert_ok!(Zk::verify_snark(
            RuntimeOrigin::signed(verifier_account),
            circuit_id,
            proof,
            inputs
        ));

        assert_eq!(Zk::total_verifications(), 1);
    });
}

// ===================================================================
// Verification limit enforcement
// ===================================================================

#[test]
fn verification_limit_enforced() {
    new_test_ext().execute_with(|| {
        // MaxVerificationsPerBlock = 100
        // Fill up to the limit using share proofs with unique witnesses
        for i in 0..100u8 {
            let witness = ShareWitness {
                share_value: [i; 32],
                share_index: i,
                randomness: [i.wrapping_add(1); 32],
            };
            let (statement, proof) = Zk::generate_share_proof(&witness);
            let bounded = BoundedVec::try_from(proof).expect("fits");
            assert_ok!(Zk::verify_share_proof(
                RuntimeOrigin::signed(1),
                statement,
                bounded
            ));
        }

        // 101st verification should fail
        let witness = ShareWitness {
            share_value: [200u8; 32],
            share_index: 200,
            randomness: [201u8; 32],
        };
        let (statement, proof) = Zk::generate_share_proof(&witness);
        let bounded = BoundedVec::try_from(proof).expect("fits");
        assert_noop!(
            Zk::verify_share_proof(RuntimeOrigin::signed(1), statement, bounded),
            Error::<Test>::TooManyVerifications
        );
    });
}

// ===================================================================
// Circuit registry state verification
// ===================================================================

#[test]
fn circuit_registry_stores_vk_hash() {
    new_test_ext().execute_with(|| {
        let circuit_id = H256([10u8; 32]);
        let vk_data = vec![42u8; 128];
        let expected_vk_hash = H256(sp_core::blake2_256(&vk_data));
        let vk = BoundedVec::try_from(vk_data).expect("fits");

        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            circuit_id,
            SnarkProofType::Groth16,
            vk
        ));

        let circuit = Zk::circuit_registry(circuit_id).expect("registered");
        assert_eq!(circuit.vk_hash, expected_vk_hash);
        assert_eq!(circuit.proof_type, SnarkProofType::Groth16);
    });
}

#[test]
fn circuit_registry_all_proof_types() {
    new_test_ext().execute_with(|| {
        let vk = BoundedVec::try_from(vec![1u8; 64]).expect("fits");

        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            H256([1u8; 32]),
            SnarkProofType::Groth16,
            vk.clone()
        ));
        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            H256([2u8; 32]),
            SnarkProofType::PlonK,
            vk.clone()
        ));
        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            H256([3u8; 32]),
            SnarkProofType::Halo2,
            vk
        ));

        assert_eq!(
            Zk::circuit_registry(H256([1u8; 32]))
                .expect("exists")
                .proof_type,
            SnarkProofType::Groth16
        );
        assert_eq!(
            Zk::circuit_registry(H256([2u8; 32]))
                .expect("exists")
                .proof_type,
            SnarkProofType::PlonK
        );
        assert_eq!(
            Zk::circuit_registry(H256([3u8; 32]))
                .expect("exists")
                .proof_type,
            SnarkProofType::Halo2
        );
    });
}

// ===================================================================
// Circuit validation and deregistration
// ===================================================================

#[test]
fn register_circuit_rejects_small_vk() {
    new_test_ext().execute_with(|| {
        let circuit_id = H256([10u8; 32]);
        // VK smaller than MIN_VK_SIZE (32 bytes)
        let small_vk = BoundedVec::try_from(vec![1u8; 16]).expect("fits");

        assert_noop!(
            Zk::register_circuit(
                RuntimeOrigin::root(),
                circuit_id,
                SnarkProofType::Groth16,
                small_vk
            ),
            Error::<Test>::InvalidVerificationKey
        );
    });
}

#[test]
fn register_circuit_sets_version_and_active() {
    new_test_ext().execute_with(|| {
        let circuit_id = H256([10u8; 32]);
        let vk = BoundedVec::try_from(vec![1u8; 64]).expect("fits");

        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            circuit_id,
            SnarkProofType::Groth16,
            vk
        ));

        let circuit = Zk::circuit_registry(circuit_id).expect("registered");
        assert_eq!(circuit.version, 1);
        assert!(circuit.active);
    });
}

#[test]
fn deregister_circuit_success() {
    new_test_ext().execute_with(|| {
        let circuit_id = H256([10u8; 32]);
        let vk = BoundedVec::try_from(vec![1u8; 64]).expect("fits");

        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            circuit_id,
            SnarkProofType::Groth16,
            vk
        ));

        assert_ok!(Zk::deregister_circuit(RuntimeOrigin::root(), circuit_id));

        let circuit = Zk::circuit_registry(circuit_id).expect("still exists");
        assert!(!circuit.active);
    });
}

#[test]
fn deregister_circuit_requires_root() {
    new_test_ext().execute_with(|| {
        let circuit_id = H256([10u8; 32]);
        let vk = BoundedVec::try_from(vec![1u8; 64]).expect("fits");

        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            circuit_id,
            SnarkProofType::Groth16,
            vk
        ));

        assert_noop!(
            Zk::deregister_circuit(RuntimeOrigin::signed(1), circuit_id),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn deregister_nonexistent_circuit_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Zk::deregister_circuit(RuntimeOrigin::root(), H256([99u8; 32])),
            Error::<Test>::CircuitNotFound
        );
    });
}

#[test]
fn deregister_already_inactive_circuit_fails() {
    new_test_ext().execute_with(|| {
        let circuit_id = H256([10u8; 32]);
        let vk = BoundedVec::try_from(vec![1u8; 64]).expect("fits");

        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            circuit_id,
            SnarkProofType::Groth16,
            vk
        ));
        assert_ok!(Zk::deregister_circuit(RuntimeOrigin::root(), circuit_id));

        assert_noop!(
            Zk::deregister_circuit(RuntimeOrigin::root(), circuit_id),
            Error::<Test>::CircuitNotActive
        );
    });
}

#[test]
fn verify_snark_on_deregistered_circuit_fails() {
    new_test_ext().execute_with(|| {
        let verifier_account = 1u64;
        let verifier = account_to_actor(verifier_account);
        assert_ok!(Zk::add_trusted_verifier(RuntimeOrigin::root(), verifier));

        let circuit_id = H256([10u8; 32]);
        let vk = BoundedVec::try_from(vec![1u8; 256]).expect("fits");
        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            circuit_id,
            SnarkProofType::Groth16,
            vk
        ));

        // Deregister the circuit
        assert_ok!(Zk::deregister_circuit(RuntimeOrigin::root(), circuit_id));

        // Try to verify against deregistered circuit
        let proof = BoundedVec::try_from(vec![1u8; 256]).expect("fits");
        let inputs = BoundedVec::try_from(vec![[1u8; 32]]).expect("fits");

        assert_noop!(
            Zk::verify_snark(
                RuntimeOrigin::signed(verifier_account),
                circuit_id,
                proof,
                inputs
            ),
            Error::<Test>::CircuitNotActive
        );
    });
}

// ===================================================================
// Proof system mode transition tests
// ===================================================================

#[test]
fn transition_mode_legacy_to_transitional() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            Zk::proof_system_mode(),
            crate::migration::ProofSystemMode::Legacy
        );

        assert_ok!(Zk::transition_proof_system_mode(
            RuntimeOrigin::root(),
            crate::migration::ProofSystemMode::Transitional
        ));

        assert_eq!(
            Zk::proof_system_mode(),
            crate::migration::ProofSystemMode::Transitional
        );
    });
}

#[test]
fn transition_mode_requires_root() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Zk::transition_proof_system_mode(
                RuntimeOrigin::signed(1),
                crate::migration::ProofSystemMode::Transitional
            ),
            frame_support::error::BadOrigin
        );
    });
}

#[test]
fn transition_mode_cannot_go_backward() {
    new_test_ext().execute_with(|| {
        assert_ok!(Zk::transition_proof_system_mode(
            RuntimeOrigin::root(),
            crate::migration::ProofSystemMode::Transitional
        ));

        assert_noop!(
            Zk::transition_proof_system_mode(
                RuntimeOrigin::root(),
                crate::migration::ProofSystemMode::Legacy
            ),
            Error::<Test>::InvalidModeTransition
        );
    });
}

#[test]
fn transition_mode_full_path() {
    new_test_ext().execute_with(|| {
        assert_ok!(Zk::transition_proof_system_mode(
            RuntimeOrigin::root(),
            crate::migration::ProofSystemMode::Transitional
        ));
        assert_ok!(Zk::transition_proof_system_mode(
            RuntimeOrigin::root(),
            crate::migration::ProofSystemMode::SnarkOnly
        ));

        assert_eq!(
            Zk::proof_system_mode(),
            crate::migration::ProofSystemMode::SnarkOnly
        );
    });
}

// ===================================================================
// Circuit registry bounds
// ===================================================================

#[test]
fn circuit_registry_bounded_by_max_circuits() {
    // Use a small test with limited MaxCircuits=256
    // Register 256 circuits, then the 257th should fail
    new_test_ext().execute_with(|| {
        let vk = BoundedVec::try_from(vec![1u8; 64]).expect("fits");

        for i in 0..256u32 {
            let mut circuit_id = [0u8; 32];
            circuit_id[..4].copy_from_slice(&i.to_le_bytes());
            assert_ok!(Zk::register_circuit(
                RuntimeOrigin::root(),
                H256(circuit_id),
                SnarkProofType::Groth16,
                vk.clone()
            ));
        }

        assert_eq!(Zk::circuit_count(), 256);

        // 257th should fail
        let mut overflow_id = [0u8; 32];
        overflow_id[..4].copy_from_slice(&256u32.to_le_bytes());
        assert_noop!(
            Zk::register_circuit(
                RuntimeOrigin::root(),
                H256(overflow_id),
                SnarkProofType::Groth16,
                vk
            ),
            Error::<Test>::CircuitRegistryFull
        );
    });
}

#[test]
fn deregister_decrements_circuit_count() {
    new_test_ext().execute_with(|| {
        let circuit_id = H256([10u8; 32]);
        let vk = BoundedVec::try_from(vec![1u8; 64]).expect("fits");

        assert_ok!(Zk::register_circuit(
            RuntimeOrigin::root(),
            circuit_id,
            SnarkProofType::Groth16,
            vk
        ));
        assert_eq!(Zk::circuit_count(), 1);

        assert_ok!(Zk::deregister_circuit(RuntimeOrigin::root(), circuit_id));
        assert_eq!(Zk::circuit_count(), 0);
    });
}
