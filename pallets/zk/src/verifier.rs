//! ZK Verifier trait and implementations.
//!
//! This module defines the pluggable verification architecture for the ZK pallet.
//! Implementations range from stub verifiers (for testing) to production-ready
//! cryptographic verifiers (Groth16, PlonK, Halo2).
//!
//! # Verifier Types
//!
//! - [`StubVerifier`]: Hash-based stub that exposes secrets on-chain. Testing only.
//! - [`ConfigurableVerifier`]: Returns a configurable result. For test harnesses.
//! - [`NullVerifier`]: Always rejects. For disabled proof types or migration.
//!
//! # Migration Path
//!
//! `StubVerifier` -> `Groth16Verifier` (v0.8.20) via `ProofSystemMode` in migration.rs.

use alloc::vec::Vec;
use seveny_primitives::crypto::{hash_with_domain, DOMAIN_NULLIFIER};
use seveny_primitives::traits::ConstantTimeEq;
use sp_core::{blake2_256, H256};

use crate::{
    AccessStatement, PresenceStatement, ShareStatement, DOMAIN_ACCESS_PROOF, DOMAIN_SHARE_PROOF,
};

/// Trait for zero-knowledge proof verification.
///
/// Implementations must verify that a proof is valid for the given statement
/// without requiring knowledge of the witness (private input).
///
/// # Invariants
///
/// - **INV73:** Share proofs must verify against commitments
/// - **INV74:** Presence proofs must verify statement validity
/// - **INV75:** Access proofs must verify authorization
pub trait ZkVerifier {
    /// Verify a share proof against a commitment hash (INV73).
    fn verify_share_proof(statement: &ShareStatement, proof: &[u8]) -> bool;

    /// Verify a presence proof with nullifier binding (INV74).
    fn verify_presence_proof(statement: &PresenceStatement, proof: &[u8]) -> bool;

    /// Verify an access proof for vault authorization (INV75).
    fn verify_access_proof(statement: &AccessStatement, proof: &[u8]) -> bool;

    /// Verify a SNARK proof against a circuit. Default: reject all.
    fn verify_snark(_proof: &[u8], _inputs: &[[u8; 32]], _vk: &[u8]) -> bool {
        false
    }
}

/// Hash-based stub verifier for development and testing.
///
/// **SECURITY WARNING:** This verifier requires raw secrets in the proof data,
/// which means secrets are exposed on-chain in extrinsic call data. This
/// completely defeats ZK privacy. Replace with a real ZK circuit (e.g. Groth16)
/// before any production deployment.
pub struct StubVerifier;

impl ZkVerifier for StubVerifier {
    fn verify_share_proof(statement: &ShareStatement, proof: &[u8]) -> bool {
        const SHARE_PROOF_SIZE: usize = 65;
        if proof.len() != SHARE_PROOF_SIZE {
            return false;
        }

        let Ok(share_value): Result<[u8; 32], _> = proof[0..32].try_into() else {
            return false;
        };
        let share_index = proof[32];
        let Ok(randomness): Result<[u8; 32], _> = proof[33..65].try_into() else {
            return false;
        };

        let mut input = Vec::with_capacity(DOMAIN_SHARE_PROOF.len() + 65);
        input.extend_from_slice(DOMAIN_SHARE_PROOF);
        input.extend_from_slice(&share_value);
        input.push(share_index);
        input.extend_from_slice(&randomness);
        let computed = H256(blake2_256(&input));

        computed.ct_eq(&statement.commitment_hash)
    }

    /// INV74: Proof data contains secret_commitment and nullifier_binding,
    /// NOT the raw secret. Verifies the nullifier_binding matches the
    /// statement's nullifier and epoch. Secret commitment checked for
    /// non-zero. Actual ZK verification deferred to Groth16 in v0.8.20.
    fn verify_presence_proof(statement: &PresenceStatement, proof: &[u8]) -> bool {
        const PRESENCE_PROOF_SIZE: usize = 80;
        if proof.len() != PRESENCE_PROOF_SIZE {
            return false;
        }

        let Ok(secret_commitment): Result<[u8; 32], _> = proof[0..32].try_into() else {
            return false;
        };
        let Ok(nullifier_binding): Result<[u8; 32], _> = proof[32..64].try_into() else {
            return false;
        };

        // Reject zero commitment (no valid secret hashes to all-zero)
        if secret_commitment == [0u8; 32] {
            return false;
        }

        // Verify nullifier_binding = H(DOMAIN_NULLIFIER || nullifier || epoch_id)
        let mut expected_input = Vec::with_capacity(40);
        expected_input.extend_from_slice(statement.nullifier.0.as_bytes());
        expected_input.extend_from_slice(&statement.epoch_id.to_le_bytes());
        let expected_binding = hash_with_domain(DOMAIN_NULLIFIER, &expected_input);

        expected_binding.ct_eq(&H256(nullifier_binding))
    }

    fn verify_access_proof(statement: &AccessStatement, proof: &[u8]) -> bool {
        const ACCESS_PROOF_SIZE: usize = 68;
        if proof.len() != ACCESS_PROOF_SIZE {
            return false;
        }

        let Ok(actor_bytes): Result<[u8; 32], _> = proof[0..32].try_into() else {
            return false;
        };
        let Ok(ring_position_bytes): Result<[u8; 4], _> = proof[32..36].try_into() else {
            return false;
        };
        let Ok(membership): Result<[u8; 32], _> = proof[36..68].try_into() else {
            return false;
        };

        let mut input = Vec::with_capacity(DOMAIN_ACCESS_PROOF.len() + 76);
        input.extend_from_slice(DOMAIN_ACCESS_PROOF);
        input.extend_from_slice(&statement.vault_id.to_le_bytes());
        input.extend_from_slice(&actor_bytes);
        input.extend_from_slice(&ring_position_bytes);
        input.extend_from_slice(&membership);
        let computed = H256(blake2_256(&input));

        computed.ct_eq(&statement.access_hash)
    }

    fn verify_snark(proof: &[u8], inputs: &[[u8; 32]], _vk: &[u8]) -> bool {
        log::warn!(
            target: "pallet-zk",
            "STUB: SNARK verifier — no cryptographic check"
        );
        proof.len() >= 192 && !inputs.is_empty()
    }
}

/// Configurable verifier for testing. Returns a fixed result.
///
/// Used in test harnesses to simulate verification success/failure
/// without depending on actual cryptographic operations.
pub struct ConfigurableVerifier<const RESULT: bool>;

impl<const RESULT: bool> ZkVerifier for ConfigurableVerifier<RESULT> {
    fn verify_share_proof(_statement: &ShareStatement, _proof: &[u8]) -> bool {
        RESULT
    }

    fn verify_presence_proof(_statement: &PresenceStatement, _proof: &[u8]) -> bool {
        RESULT
    }

    fn verify_access_proof(_statement: &AccessStatement, _proof: &[u8]) -> bool {
        RESULT
    }

    fn verify_snark(_proof: &[u8], _inputs: &[[u8; 32]], _vk: &[u8]) -> bool {
        RESULT
    }
}

/// Null verifier that always rejects all proofs.
///
/// Used during migration when a proof type is disabled, or as a safety
/// fallback when the verification system is in an invalid state.
pub type NullVerifier = ConfigurableVerifier<false>;

/// Accept-all verifier for genesis / testing bootstrapping.
pub type AcceptAllVerifier = ConfigurableVerifier<true>;

#[cfg(test)]
mod verifier_tests {
    use super::*;
    use seveny_primitives::crypto::{hash_with_domain, Nullifier, StateRoot, DOMAIN_NULLIFIER};

    #[test]
    fn stub_verifier_share_proof_roundtrip() {
        let mut input = Vec::new();
        input.extend_from_slice(DOMAIN_SHARE_PROOF);
        input.extend_from_slice(&[1u8; 32]);
        input.push(0u8);
        input.extend_from_slice(&[2u8; 32]);
        let commitment_hash = H256(blake2_256(&input));

        let statement = ShareStatement { commitment_hash };

        let mut proof = Vec::with_capacity(65);
        proof.extend_from_slice(&[1u8; 32]);
        proof.push(0u8);
        proof.extend_from_slice(&[2u8; 32]);

        assert!(StubVerifier::verify_share_proof(&statement, &proof));
    }

    #[test]
    fn stub_verifier_rejects_wrong_share() {
        let statement = ShareStatement {
            commitment_hash: H256([0xff; 32]),
        };
        let proof = vec![0u8; 65];
        assert!(!StubVerifier::verify_share_proof(&statement, &proof));
    }

    #[test]
    fn stub_verifier_rejects_wrong_size() {
        let statement = ShareStatement {
            commitment_hash: H256([0; 32]),
        };
        assert!(!StubVerifier::verify_share_proof(&statement, &[0u8; 32]));
        assert!(!StubVerifier::verify_share_proof(&statement, &[0u8; 66]));
    }

    #[test]
    fn stub_verifier_presence_roundtrip() {
        let secret = [3u8; 32];
        let epoch_id = 1u64;
        let nullifier = Nullifier::derive(&secret, epoch_id);

        let statement = PresenceStatement {
            epoch_id,
            state_root: StateRoot::EMPTY,
            nullifier,
        };

        // Proof: secret_commitment[32] || nullifier_binding[32] || reserved[16]
        let secret_commitment = H256(blake2_256(&secret));
        let mut null_input = Vec::with_capacity(40);
        null_input.extend_from_slice(nullifier.0.as_bytes());
        null_input.extend_from_slice(&epoch_id.to_le_bytes());
        let nullifier_binding = hash_with_domain(DOMAIN_NULLIFIER, &null_input);

        let mut proof = Vec::with_capacity(80);
        proof.extend_from_slice(secret_commitment.as_bytes());
        proof.extend_from_slice(nullifier_binding.as_bytes());
        proof.extend_from_slice(&[0u8; 16]);

        assert!(StubVerifier::verify_presence_proof(&statement, &proof));
    }

    #[test]
    fn stub_verifier_rejects_wrong_nullifier_binding() {
        let statement = PresenceStatement {
            epoch_id: 1,
            state_root: StateRoot::EMPTY,
            nullifier: Nullifier(H256([0xff; 32])),
        };

        // Non-zero commitment but wrong nullifier binding
        let mut proof = Vec::with_capacity(80);
        proof.extend_from_slice(&[3u8; 32]); // non-zero commitment
        proof.extend_from_slice(&[0u8; 32]); // wrong binding
        proof.extend_from_slice(&[0u8; 16]);

        assert!(!StubVerifier::verify_presence_proof(&statement, &proof));
    }

    #[test]
    fn stub_verifier_rejects_zero_commitment() {
        let secret = [3u8; 32];
        let epoch_id = 1u64;
        let nullifier = Nullifier::derive(&secret, epoch_id);

        let statement = PresenceStatement {
            epoch_id,
            state_root: StateRoot::EMPTY,
            nullifier,
        };

        // Zero commitment should be rejected (INV74)
        let mut proof = Vec::with_capacity(80);
        proof.extend_from_slice(&[0u8; 32]); // zero commitment
        proof.extend_from_slice(&[0u8; 32]);
        proof.extend_from_slice(&[0u8; 16]);

        assert!(!StubVerifier::verify_presence_proof(&statement, &proof));
    }

    #[test]
    fn stub_verifier_proof_does_not_contain_raw_secret() {
        let secret = [42u8; 32];
        let epoch_id = 5u64;
        let nullifier = Nullifier::derive(&secret, epoch_id);

        let statement = PresenceStatement {
            epoch_id,
            state_root: StateRoot::EMPTY,
            nullifier,
        };

        // Build valid proof
        let secret_commitment = H256(blake2_256(&secret));
        let mut null_input = Vec::with_capacity(40);
        null_input.extend_from_slice(nullifier.0.as_bytes());
        null_input.extend_from_slice(&epoch_id.to_le_bytes());
        let nullifier_binding = hash_with_domain(DOMAIN_NULLIFIER, &null_input);

        let mut proof = Vec::with_capacity(80);
        proof.extend_from_slice(secret_commitment.as_bytes());
        proof.extend_from_slice(nullifier_binding.as_bytes());
        proof.extend_from_slice(&[0u8; 16]);

        // The raw secret must NOT appear anywhere in the proof
        assert!(
            !proof.windows(32).any(|w| w == secret),
            "Raw secret found in proof data — INV74 violation"
        );

        // But the proof should still verify
        assert!(StubVerifier::verify_presence_proof(&statement, &proof));
    }

    #[test]
    fn null_verifier_rejects_all() {
        let share_st = ShareStatement {
            commitment_hash: H256([0; 32]),
        };
        assert!(!NullVerifier::verify_share_proof(&share_st, &[0u8; 65]));

        let presence_st = PresenceStatement {
            epoch_id: 1,
            state_root: StateRoot::EMPTY,
            nullifier: Nullifier(H256([0; 32])),
        };
        assert!(!NullVerifier::verify_presence_proof(
            &presence_st,
            &[0u8; 80]
        ));

        let access_st = AccessStatement {
            vault_id: 1,
            access_hash: H256([0; 32]),
        };
        assert!(!NullVerifier::verify_access_proof(&access_st, &[0u8; 68]));

        assert!(!NullVerifier::verify_snark(&[0u8; 256], &[[0; 32]], &[]));
    }

    #[test]
    fn accept_all_verifier_accepts_all() {
        let share_st = ShareStatement {
            commitment_hash: H256([0; 32]),
        };
        assert!(AcceptAllVerifier::verify_share_proof(&share_st, &[0u8; 65]));

        let presence_st = PresenceStatement {
            epoch_id: 1,
            state_root: StateRoot::EMPTY,
            nullifier: Nullifier(H256([0; 32])),
        };
        assert!(AcceptAllVerifier::verify_presence_proof(
            &presence_st,
            &[0u8; 80]
        ));
        assert!(AcceptAllVerifier::verify_snark(&[], &[], &[]));
    }

    #[test]
    fn configurable_verifier_const_generics() {
        let st = ShareStatement {
            commitment_hash: H256([0; 32]),
        };
        assert!(ConfigurableVerifier::<true>::verify_share_proof(
            &st, &[0u8; 65]
        ));
        assert!(!ConfigurableVerifier::<false>::verify_share_proof(
            &st, &[0u8; 65]
        ));
    }
}
