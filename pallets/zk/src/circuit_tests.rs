//! End-to-end Groth16 prove/verify tests for all ZK circuits.
//!
//! Each test performs the full workflow:
//! 1. Create a blank circuit for trusted setup
//! 2. Generate proving key and verification key
//! 3. Create a circuit with witness values
//! 4. Generate a Groth16 proof
//! 5. Verify the proof against public inputs

#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use ark_bn254::{Bn254, Fr};
use ark_groth16::Groth16;
use ark_snark::SNARK;
use ark_std::rand::{rngs::StdRng, SeedableRng};

use crate::circuits::{
    access::AccessCircuit, position::PositionProximityCircuit,
    presence::PresenceCircuit, share::ShareCircuit,
};

fn test_rng() -> StdRng {
    StdRng::seed_from_u64(0)
}

// ─── Share Circuit E2E ───────────────────────────────────────────────────

#[test]
fn share_circuit_groth16_prove_verify() {
    let mut rng = test_rng();

    let value = Fr::from(42u64);
    let index = Fr::from(1u64);
    let randomness = Fr::from(0xDEADBEEFu64);

    // Trusted setup with blank circuit
    let blank = ShareCircuit::blank();
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
        .expect("setup failed");

    // Create circuit with witness
    let circuit = ShareCircuit::new(value, index, randomness);
    let public_inputs = circuit.public_inputs();

    // Prove
    let proof =
        Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove failed");

    // Verify
    let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
        .expect("verify failed");
    assert!(valid, "valid proof should verify");
}

#[test]
fn share_circuit_groth16_rejects_wrong_inputs() {
    let mut rng = test_rng();

    let value = Fr::from(42u64);
    let index = Fr::from(1u64);
    let randomness = Fr::from(0xDEADBEEFu64);

    let blank = ShareCircuit::blank();
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
        .expect("setup failed");

    let circuit = ShareCircuit::new(value, index, randomness);
    let proof =
        Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove failed");

    // Wrong public input
    let wrong_inputs = vec![Fr::from(999u64)];
    let valid = Groth16::<Bn254>::verify(&vk, &wrong_inputs, &proof)
        .expect("verify failed");
    assert!(!valid, "wrong public input should fail");
}

#[test]
fn share_circuit_different_witnesses_different_commitments() {
    let mut rng = test_rng();

    let blank = ShareCircuit::blank();
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
        .expect("setup failed");

    let circuit1 =
        ShareCircuit::new(Fr::from(1u64), Fr::from(1u64), Fr::from(1u64));
    let circuit2 =
        ShareCircuit::new(Fr::from(2u64), Fr::from(1u64), Fr::from(1u64));

    let inputs1 = circuit1.public_inputs();
    let inputs2 = circuit2.public_inputs();
    assert_ne!(inputs1, inputs2, "different values should give different commitments");

    let proof1 = Groth16::<Bn254>::prove(&pk, circuit1, &mut rng)
        .expect("prove failed");
    let proof2 = Groth16::<Bn254>::prove(&pk, circuit2, &mut rng)
        .expect("prove failed");

    assert!(Groth16::<Bn254>::verify(&vk, &inputs1, &proof1).unwrap());
    assert!(Groth16::<Bn254>::verify(&vk, &inputs2, &proof2).unwrap());

    // Cross-verify should fail
    assert!(!Groth16::<Bn254>::verify(&vk, &inputs1, &proof2).unwrap());
}

// ─── Presence Circuit E2E ────────────────────────────────────────────────

#[test]
fn presence_circuit_groth16_prove_verify() {
    let mut rng = test_rng();
    let depth = 5;

    let secret = Fr::from(12345u64);
    let epoch_id = 1u64;
    let randomness = Fr::from(0xCAFEu64);

    let siblings: Vec<Fr> =
        (0..depth).map(|i| Fr::from((i + 100) as u64)).collect();
    let path_bits: Vec<bool> =
        (0..depth).map(|i| i % 2 == 0).collect();

    // Setup with matching depth
    let blank = PresenceCircuit::blank(depth);
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
        .expect("setup failed");

    // Prove
    let circuit = PresenceCircuit::new(
        secret,
        epoch_id,
        randomness,
        siblings,
        path_bits,
    );
    let public_inputs = circuit.public_inputs();
    let proof =
        Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove failed");

    // Verify
    let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
        .expect("verify failed");
    assert!(valid, "valid presence proof should verify");
}

#[test]
fn presence_circuit_groth16_rejects_wrong_nullifier() {
    let mut rng = test_rng();
    let depth = 5;

    let secret = Fr::from(12345u64);
    let epoch_id = 1u64;
    let randomness = Fr::from(0xCAFEu64);

    let siblings: Vec<Fr> =
        (0..depth).map(|i| Fr::from((i + 100) as u64)).collect();
    let path_bits: Vec<bool> =
        (0..depth).map(|i| i % 2 == 0).collect();

    let blank = PresenceCircuit::blank(depth);
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
        .expect("setup failed");

    let circuit = PresenceCircuit::new(
        secret,
        epoch_id,
        randomness,
        siblings,
        path_bits,
    );
    let mut public_inputs = circuit.public_inputs();
    let proof =
        Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove failed");

    // Tamper with nullifier
    public_inputs[0] = Fr::from(999u64);
    let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
        .expect("verify failed");
    assert!(!valid, "wrong nullifier should fail");
}

// ─── Access Circuit E2E ──────────────────────────────────────────────────

#[test]
fn access_circuit_groth16_prove_verify() {
    let mut rng = test_rng();
    let depth = 5;

    let actor_id = Fr::from(0xABCDu64);
    let vault_id = 7u64;
    let ring_position = Fr::from(3u64);

    let siblings: Vec<Fr> =
        (0..depth).map(|i| Fr::from((i + 200) as u64)).collect();
    let path_bits: Vec<bool> =
        (0..depth).map(|i| i % 3 == 0).collect();

    let blank = AccessCircuit::blank(depth);
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
        .expect("setup failed");

    let circuit = AccessCircuit::new(
        actor_id,
        vault_id,
        ring_position,
        siblings,
        path_bits,
    );
    let public_inputs = circuit.public_inputs();
    let proof =
        Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove failed");

    let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
        .expect("verify failed");
    assert!(valid, "valid access proof should verify");
}

#[test]
fn access_circuit_groth16_rejects_wrong_vault() {
    let mut rng = test_rng();
    let depth = 5;

    let actor_id = Fr::from(0xABCDu64);
    let vault_id = 7u64;
    let ring_position = Fr::from(3u64);

    let siblings: Vec<Fr> =
        (0..depth).map(|i| Fr::from((i + 200) as u64)).collect();
    let path_bits: Vec<bool> =
        (0..depth).map(|i| i % 3 == 0).collect();

    let blank = AccessCircuit::blank(depth);
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
        .expect("setup failed");

    let circuit = AccessCircuit::new(
        actor_id,
        vault_id,
        ring_position,
        siblings,
        path_bits,
    );
    let mut public_inputs = circuit.public_inputs();
    let proof =
        Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove failed");

    // Tamper with vault_id
    public_inputs[0] = Fr::from(999u64);
    let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
        .expect("verify failed");
    assert!(!valid, "wrong vault_id should fail");
}

// ─── Position Proximity Circuit E2E ──────────────────────────────────────

#[test]
fn position_circuit_groth16_prove_verify() {
    let mut rng = test_rng();

    let blank = PositionProximityCircuit::blank();
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
        .expect("setup failed");

    // Position (101, 102) within radius_sq=2500 of center (100, 100)
    let circuit =
        PositionProximityCircuit::new(101, 102, 100, 100, 2500, 1)
            .expect("position within radius");
    let public_inputs = circuit.public_inputs();
    let proof =
        Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove failed");

    let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
        .expect("verify failed");
    assert!(valid, "valid position proof should verify");
}

#[test]
fn position_circuit_groth16_boundary_exact() {
    let mut rng = test_rng();

    let blank = PositionProximityCircuit::blank();
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
        .expect("setup failed");

    // Distance = sqrt(9+16) = 5, radius_sq = 25 (exactly at boundary)
    let circuit =
        PositionProximityCircuit::new(103, 104, 100, 100, 25, 1)
            .expect("position at boundary");
    let public_inputs = circuit.public_inputs();
    let proof =
        Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove failed");

    let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
        .expect("verify failed");
    assert!(valid, "boundary position proof should verify");
}

#[test]
fn position_circuit_groth16_rejects_wrong_commitment() {
    let mut rng = test_rng();

    let blank = PositionProximityCircuit::blank();
    let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
        .expect("setup failed");

    let circuit =
        PositionProximityCircuit::new(101, 102, 100, 100, 2500, 1)
            .expect("position within radius");
    let mut public_inputs = circuit.public_inputs();
    let proof =
        Groth16::<Bn254>::prove(&pk, circuit, &mut rng).expect("prove failed");

    // Tamper with region commitment
    public_inputs[0] = Fr::from(999u64);
    let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof)
        .expect("verify failed");
    assert!(!valid, "wrong region commitment should fail");
}
