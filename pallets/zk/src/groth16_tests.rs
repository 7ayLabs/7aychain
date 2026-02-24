//! Groth16 verifier integration tests.
//!
//! Uses std-enabled arkworks in dev-dependencies to generate proofs
//! off-chain and verify them using the on-chain Groth16Verifier.

#![cfg(all(test, feature = "groth16"))]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use ark_bn254::{Bn254, Fr};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::Groth16;
use ark_r1cs_std::{alloc::AllocVar, eq::EqGadget, fields::fp::FpVar};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::CanonicalSerialize;
use ark_snark::SNARK;
use ark_std::rand::thread_rng;

use crate::groth16::Groth16Verifier;
use crate::verifier::ZkVerifier;

/// Trivial test circuit: proves knowledge of x such that x * x == y.
/// Public input: y
/// Private input (witness): x
#[derive(Clone)]
struct SquareCircuit {
    x: Option<Fr>,
    y: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for SquareCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let x_var = FpVar::new_witness(cs.clone(), || {
            self.x.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let y_var = FpVar::new_input(cs, || self.y.ok_or(SynthesisError::AssignmentMissing))?;

        let x_squared = &x_var * &x_var;
        x_squared.enforce_equal(&y_var)?;

        Ok(())
    }
}

/// Generate proof, VK, and public inputs for x * x == y.
fn generate_test_vectors(x_val: Fr) -> (Vec<u8>, Vec<u8>, Vec<[u8; 32]>) {
    let y_val = x_val * x_val;
    let circuit = SquareCircuit {
        x: Some(x_val),
        y: Some(y_val),
    };
    let mut rng = thread_rng();

    let (pk, vk) =
        Groth16::<Bn254>::circuit_specific_setup(SquareCircuit { x: None, y: None }, &mut rng)
            .expect("setup failed");

    let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove failed");

    let mut proof_bytes = Vec::new();
    proof
        .serialize_compressed(&mut proof_bytes)
        .expect("proof serialize failed");

    let mut vk_bytes = Vec::new();
    vk.serialize_compressed(&mut vk_bytes)
        .expect("vk serialize failed");

    let y_be = y_val.into_bigint().to_bytes_be();
    let mut y_bytes = [0u8; 32];
    let offset = 32_usize.saturating_sub(y_be.len());
    y_bytes[offset..].copy_from_slice(&y_be[..32.min(y_be.len())]);

    (proof_bytes, vk_bytes, vec![y_bytes])
}

#[test]
fn groth16_verify_valid_proof() {
    let x = Fr::from(3u64);
    let (proof, vk, inputs) = generate_test_vectors(x);

    assert!(
        Groth16Verifier::verify_snark(&proof, &inputs, &vk),
        "Valid Groth16 proof should verify"
    );
}

#[test]
fn groth16_reject_wrong_public_input() {
    let x = Fr::from(3u64);
    let (proof, vk, _) = generate_test_vectors(x);

    let wrong_y = Fr::from(10u64);
    let wrong_be = wrong_y.into_bigint().to_bytes_be();
    let mut wrong_bytes = [0u8; 32];
    let offset = 32_usize.saturating_sub(wrong_be.len());
    wrong_bytes[offset..].copy_from_slice(&wrong_be[..32.min(wrong_be.len())]);

    assert!(
        !Groth16Verifier::verify_snark(&proof, &[wrong_bytes], &vk),
        "Wrong public input should fail verification"
    );
}

#[test]
fn groth16_reject_tampered_proof() {
    let x = Fr::from(5u64);
    let (mut proof, vk, inputs) = generate_test_vectors(x);

    if let Some(byte) = proof.get_mut(10) {
        *byte ^= 0xFF;
    }

    assert!(
        !Groth16Verifier::verify_snark(&proof, &inputs, &vk),
        "Tampered proof should fail"
    );
}

#[test]
fn groth16_reject_empty_proof() {
    let x = Fr::from(3u64);
    let (_, vk, inputs) = generate_test_vectors(x);

    assert!(
        !Groth16Verifier::verify_snark(&[], &inputs, &vk),
        "Empty proof should fail"
    );
}

#[test]
fn groth16_reject_too_short_proof() {
    let x = Fr::from(3u64);
    let (proof, vk, inputs) = generate_test_vectors(x);

    let short = &proof[..64];
    assert!(
        !Groth16Verifier::verify_snark(short, &inputs, &vk),
        "Truncated proof should fail"
    );
}

#[test]
fn groth16_reject_wrong_vk() {
    let x = Fr::from(3u64);
    let (proof, _, inputs) = generate_test_vectors(x);

    // Different trusted setup produces different VK
    let (_, wrong_vk, _) = generate_test_vectors(Fr::from(7u64));

    assert!(
        !Groth16Verifier::verify_snark(&proof, &inputs, &wrong_vk),
        "Proof against wrong VK should fail"
    );
}

#[test]
fn groth16_verify_multiple_witnesses() {
    for x_val in [1u64, 2, 7, 100, 999] {
        let x = Fr::from(x_val);
        let (proof, vk, inputs) = generate_test_vectors(x);

        assert!(
            Groth16Verifier::verify_snark(&proof, &inputs, &vk),
            "Valid proof for x={x_val} should verify"
        );
    }
}

#[test]
fn groth16_proof_size_is_128_bytes() {
    let x = Fr::from(3u64);
    let (proof, _, _) = generate_test_vectors(x);

    assert_eq!(
        proof.len(),
        128,
        "Compressed Groth16 BN254 proof should be 128 bytes"
    );
}

#[test]
fn groth16_vk_size_reasonable() {
    let x = Fr::from(3u64);
    let (_, vk, _) = generate_test_vectors(x);

    assert!(
        vk.len() >= 200,
        "VK should be at least 200 bytes, got {}",
        vk.len()
    );
    assert!(
        vk.len() <= 4096,
        "VK should fit in MaxVkSize (4096), got {}",
        vk.len()
    );
}
