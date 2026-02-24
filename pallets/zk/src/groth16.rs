//! Groth16 BN254 verifier implementation.
//!
//! Provides on-chain verification of Groth16 proofs over the BN254
//! (alt_bn128) curve using the arkworks library. Proof generation
//! remains off-chain.
//!
//! # Proof Format (compressed, 128 bytes)
//!
//! | Field | Size | Encoding |
//! |-------|------|----------|
//! | pi_a  | 32B  | G1 compressed |
//! | pi_b  | 64B  | G2 compressed |
//! | pi_c  | 32B  | G1 compressed |
//!
//! # Public Inputs
//!
//! Each `[u8; 32]` is interpreted as a big-endian BN254 scalar field
//! element (Fr). Values exceeding Fr::MODULUS are reduced mod p.

extern crate alloc;

use alloc::vec::Vec;
use ark_bn254::{Bn254, Fr};
use ark_ff::PrimeField;
use ark_groth16::{PreparedVerifyingKey, Proof, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use ark_snark::SNARK;

use crate::{AccessStatement, PresenceStatement, ShareStatement};

/// Minimum size for a valid compressed Groth16 BN254 proof.
pub const GROTH16_MIN_PROOF_SIZE: usize = 128;

/// Errors during Groth16 deserialization or verification.
#[derive(Debug)]
pub enum Groth16Error {
    /// Proof bytes could not be deserialized.
    InvalidProof,
    /// Verification key bytes could not be deserialized.
    InvalidVerificationKey,
    /// Pairing check failed or verification returned an error.
    VerificationFailed,
}

/// Groth16 BN254 verifier for production SNARK proof verification.
///
/// Implements `ZkVerifier` with real pairing-based cryptographic
/// verification for `verify_snark`. Legacy proof methods (share,
/// presence, access) delegate to `StubVerifier` since no R1CS
/// circuits exist for those yet.
pub struct Groth16Verifier;

impl Groth16Verifier {
    /// Deserialize a Groth16 proof from bytes.
    /// Tries compressed format first, falls back to uncompressed.
    fn deserialize_proof(proof_bytes: &[u8]) -> Result<Proof<Bn254>, Groth16Error> {
        Proof::<Bn254>::deserialize_compressed(proof_bytes)
            .or_else(|_| Proof::<Bn254>::deserialize_uncompressed(proof_bytes))
            .map_err(|_| Groth16Error::InvalidProof)
    }

    /// Deserialize a verification key from bytes.
    /// Tries compressed format first, falls back to uncompressed.
    fn deserialize_vk(vk_bytes: &[u8]) -> Result<VerifyingKey<Bn254>, Groth16Error> {
        VerifyingKey::<Bn254>::deserialize_compressed(vk_bytes)
            .or_else(|_| VerifyingKey::<Bn254>::deserialize_uncompressed(vk_bytes))
            .map_err(|_| Groth16Error::InvalidVerificationKey)
    }

    /// Convert `[u8; 32]` public inputs to BN254 Fr field elements.
    /// Each input is interpreted as big-endian and reduced mod p.
    fn convert_public_inputs(inputs: &[[u8; 32]]) -> Vec<Fr> {
        inputs
            .iter()
            .map(|bytes| Fr::from_be_bytes_mod_order(bytes))
            .collect()
    }

    /// Core Groth16 verification: deserialize, prepare VK, verify.
    fn verify_inner(
        proof_bytes: &[u8],
        input_bytes: &[[u8; 32]],
        vk_bytes: &[u8],
    ) -> Result<bool, Groth16Error> {
        let proof = Self::deserialize_proof(proof_bytes)?;
        let vk = Self::deserialize_vk(vk_bytes)?;
        let inputs = Self::convert_public_inputs(input_bytes);

        let pvk = PreparedVerifyingKey::from(vk);

        ark_groth16::Groth16::<Bn254>::verify_with_processed_vk(&pvk, &inputs, &proof)
            .map_err(|_| Groth16Error::VerificationFailed)
    }
}

impl crate::verifier::ZkVerifier for Groth16Verifier {
    /// Delegates to StubVerifier (hash-based).
    /// Replaced with ZK circuit in future version.
    fn verify_share_proof(statement: &ShareStatement, proof: &[u8]) -> bool {
        crate::StubVerifier::verify_share_proof(statement, proof)
    }

    /// Delegates to StubVerifier (hash-based).
    /// Replaced with ZK circuit in future version.
    fn verify_presence_proof(statement: &PresenceStatement, proof: &[u8]) -> bool {
        crate::StubVerifier::verify_presence_proof(statement, proof)
    }

    /// Delegates to StubVerifier (hash-based).
    /// Replaced with ZK circuit in future version.
    fn verify_access_proof(statement: &AccessStatement, proof: &[u8]) -> bool {
        crate::StubVerifier::verify_access_proof(statement, proof)
    }

    /// Verify a Groth16 BN254 SNARK proof against a verification key.
    ///
    /// Performs full pairing-based verification:
    /// 1. Deserializes proof (compressed or uncompressed)
    /// 2. Deserializes verification key
    /// 3. Converts public inputs to Fr scalars
    /// 4. Runs Groth16 pairing check
    fn verify_snark(proof: &[u8], inputs: &[[u8; 32]], vk: &[u8]) -> bool {
        if proof.len() < GROTH16_MIN_PROOF_SIZE {
            log::warn!(
                target: "pallet-zk",
                "Groth16: proof too short ({} < {})",
                proof.len(),
                GROTH16_MIN_PROOF_SIZE,
            );
            return false;
        }

        match Self::verify_inner(proof, inputs, vk) {
            Ok(valid) => {
                if !valid {
                    log::debug!(
                        target: "pallet-zk",
                        "Groth16: pairing check failed",
                    );
                }
                valid
            }
            Err(err) => {
                log::warn!(
                    target: "pallet-zk",
                    "Groth16: verification error: {:?}",
                    err,
                );
                false
            }
        }
    }
}
