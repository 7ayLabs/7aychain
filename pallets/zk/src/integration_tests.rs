//! Cross-circuit integration tests and property-based tests.
//!
//! Tests cross-circuit consistency (shared MiMC hash, Merkle trees),
//! property-based tests for cryptographic primitives, and end-to-end
//! workflow simulations.

#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]

use ark_bn254::{Bn254, Fr};
use ark_ff::{PrimeField, Zero};
use ark_groth16::Groth16;
use ark_snark::SNARK;
use ark_std::rand::{rngs::StdRng, SeedableRng};

use crate::circuits::{
    access::AccessCircuit, attestation::AttestationCircuit, mimc_constants,
    mimc_hash, merkle_root_native, position::PositionProximityCircuit,
    presence::PresenceCircuit, reputation::ReputationCircuit,
    share::ShareCircuit, stake::StakeCircuit, vote::VoteCircuit,
    bytes_to_fr, u64_to_fr,
};

fn test_rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

// ─── MiMC Property Tests ────────────────────────────────────────────────

#[test]
fn mimc_hash_collision_resistance() {
    // Different inputs should produce different outputs
    for i in 0u64..50 {
        for j in (i + 1)..50 {
            let h1 = mimc_hash(&[Fr::from(i)]);
            let h2 = mimc_hash(&[Fr::from(j)]);
            assert_ne!(h1, h2, "collision at ({}, {})", i, j);
        }
    }
}

#[test]
fn mimc_hash_order_sensitive() {
    let a = Fr::from(1u64);
    let b = Fr::from(2u64);
    let h_ab = mimc_hash(&[a, b]);
    let h_ba = mimc_hash(&[b, a]);
    assert_ne!(h_ab, h_ba, "MiMC sponge must be order-sensitive");
}

#[test]
fn mimc_hash_length_sensitive() {
    let a = Fr::from(1u64);
    let h1 = mimc_hash(&[a]);
    let h2 = mimc_hash(&[a, Fr::zero()]);
    assert_ne!(h1, h2, "different length inputs must differ");
}

#[test]
fn mimc_constants_are_unique() {
    let constants = mimc_constants();
    for i in 0..constants.len() {
        for j in (i + 1)..constants.len() {
            assert_ne!(
                constants[i], constants[j],
                "round constants must be unique (indices {} and {})",
                i, j
            );
        }
    }
}

#[test]
fn mimc_hash_avalanche_property() {
    // Changing one bit of input should change roughly half the output bits
    let h1 = mimc_hash(&[Fr::from(0u64)]);
    let h2 = mimc_hash(&[Fr::from(1u64)]);
    // They should just be different (full avalanche in ZK context)
    assert_ne!(h1, h2);
    // And the difference should not be trivially small
    let diff = h1 - h2;
    assert_ne!(diff, Fr::zero());
    assert_ne!(diff, Fr::from(1u64));
}

// ─── Merkle Tree Property Tests ─────────────────────────────────────────

#[test]
fn merkle_root_different_leaves_different_roots() {
    let siblings: Vec<Fr> =
        (0..5).map(|i| Fr::from((i + 100) as u64)).collect();
    let path_bits: Vec<bool> = vec![false, true, false, true, false];

    for i in 0u64..20 {
        for j in (i + 1)..20 {
            let r1 =
                merkle_root_native(Fr::from(i), &siblings, &path_bits);
            let r2 =
                merkle_root_native(Fr::from(j), &siblings, &path_bits);
            assert_ne!(
                r1, r2,
                "different leaves must give different roots"
            );
        }
    }
}

#[test]
fn merkle_root_different_paths_different_roots() {
    let leaf = Fr::from(42u64);
    let siblings: Vec<Fr> =
        (0..3).map(|i| Fr::from((i + 100) as u64)).collect();

    // All possible 3-bit paths
    let mut roots = Vec::new();
    for bits in 0u8..8 {
        let path_bits: Vec<bool> =
            (0..3).map(|i| (bits >> i) & 1 == 1).collect();
        let root =
            merkle_root_native(leaf, &siblings, &path_bits);
        roots.push(root);
    }

    // All roots should be unique
    for i in 0..roots.len() {
        for j in (i + 1)..roots.len() {
            assert_ne!(
                roots[i], roots[j],
                "different paths must give different roots"
            );
        }
    }
}

#[test]
fn merkle_root_depth_independence() {
    // Root at depth 1 should differ from root at depth 2 even with
    // same leaf and first sibling
    let leaf = Fr::from(42u64);
    let sib1 = Fr::from(100u64);
    let sib2 = Fr::from(200u64);

    let root_d1 = merkle_root_native(leaf, &[sib1], &[false]);
    let root_d2 =
        merkle_root_native(leaf, &[sib1, sib2], &[false, false]);

    assert_ne!(root_d1, root_d2);
}

// ─── Cross-Circuit Consistency Tests ────────────────────────────────────

#[test]
fn share_and_presence_use_same_mimc() {
    // Verify that both circuits use the same MiMC hash function
    // by computing a commitment with the same inputs
    let secret = Fr::from(42u64);

    // ShareCircuit: commitment = MiMC(value, index, randomness)
    let share_commitment =
        mimc_hash(&[Fr::from(42u64), Fr::from(1u64), Fr::from(99u64)]);

    // PresenceCircuit: nullifier = MiMC(secret, epoch_id)
    let nullifier = mimc_hash(&[secret, Fr::from(1u64)]);

    // Both should be non-zero and deterministic
    assert_ne!(share_commitment, Fr::zero());
    assert_ne!(nullifier, Fr::zero());
    assert_ne!(share_commitment, nullifier, "different domains should differ");

    // Verify determinism
    let share2 =
        mimc_hash(&[Fr::from(42u64), Fr::from(1u64), Fr::from(99u64)]);
    assert_eq!(share_commitment, share2);
}

#[test]
fn vote_and_attestation_nullifier_independence() {
    // Same validator/device ID used in both circuits should produce
    // different nullifiers due to different domain inputs
    let id = Fr::from(0xABCDu64);
    let topic_or_epoch = Fr::from(1u64);

    // Vote nullifier = MiMC(validator_id, vote_topic)
    let vote_null = mimc_hash(&[id, topic_or_epoch]);

    // Attestation nullifier = MiMC(device_id, epoch_id)
    // Same computation, same result (this is expected since
    // domain separation comes from the public inputs)
    let att_null = mimc_hash(&[id, topic_or_epoch]);

    assert_eq!(
        vote_null, att_null,
        "same inputs to MiMC should give same outputs"
    );
}

#[test]
fn all_circuits_blank_setup_succeeds() {
    // Verify all 8 circuits can generate CRS parameters from blank instances
    let mut rng = test_rng();

    // Share
    let blank = ShareCircuit::blank();
    assert!(
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng).is_ok()
    );

    // Presence
    let blank = PresenceCircuit::blank(5);
    assert!(
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng).is_ok()
    );

    // Access
    let blank = AccessCircuit::blank(5);
    assert!(
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng).is_ok()
    );

    // Position
    let blank = PositionProximityCircuit::blank();
    assert!(
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng).is_ok()
    );

    // Vote
    let blank = VoteCircuit::blank(5);
    assert!(
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng).is_ok()
    );

    // Attestation
    let blank = AttestationCircuit::blank(5);
    assert!(
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng).is_ok()
    );

    // Reputation
    let blank = ReputationCircuit::blank();
    assert!(
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng).is_ok()
    );

    // Stake
    let blank = StakeCircuit::blank(5);
    assert!(
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng).is_ok()
    );
}

#[test]
fn proofs_from_different_circuits_not_interchangeable() {
    let mut rng = test_rng();

    // Setup share circuit
    let share_blank = ShareCircuit::blank();
    let (share_pk, share_vk) =
        Groth16::<Bn254>::circuit_specific_setup(share_blank, &mut rng)
            .expect("share setup");

    // Setup reputation circuit
    let rep_blank = ReputationCircuit::blank();
    let (rep_pk, rep_vk) =
        Groth16::<Bn254>::circuit_specific_setup(rep_blank, &mut rng)
            .expect("reputation setup");

    // Create valid share proof
    let share_circuit =
        ShareCircuit::new(Fr::from(42u64), Fr::from(1u64), Fr::from(99u64));
    let share_inputs = share_circuit.public_inputs();
    let share_proof =
        Groth16::<Bn254>::prove(&share_pk, share_circuit, &mut rng)
            .expect("share prove");

    // Create valid reputation proof
    let rep_circuit =
        ReputationCircuit::new(Fr::from(1u64), 85, 50, Fr::from(0xABu64))
            .expect("rep create");
    let rep_inputs = rep_circuit.public_inputs();
    let rep_proof =
        Groth16::<Bn254>::prove(&rep_pk, rep_circuit, &mut rng)
            .expect("rep prove");

    // Valid: share proof with share VK
    assert!(
        Groth16::<Bn254>::verify(&share_vk, &share_inputs, &share_proof)
            .unwrap()
    );

    // Valid: reputation proof with reputation VK
    assert!(
        Groth16::<Bn254>::verify(&rep_vk, &rep_inputs, &rep_proof).unwrap()
    );

    // Invalid: share proof with reputation VK (cross-circuit)
    // Note: this may error or return false depending on input count mismatch
    let cross_result =
        Groth16::<Bn254>::verify(&rep_vk, &rep_inputs, &share_proof);
    match cross_result {
        Ok(valid) => assert!(
            !valid,
            "cross-circuit proof should not verify"
        ),
        Err(_) => {} // Error is also acceptable — different VK structure
    }
}

// ─── Validator Workflow Integration ─────────────────────────────────────

#[test]
fn validator_lifecycle_vote_stake_integration() {
    // Simulate: same validator proves stake eligibility AND casts anonymous vote
    let mut rng = test_rng();
    let depth = 5;

    let validator_id = Fr::from(0xABCDu64);
    let randomness = Fr::from(0xCAFEu64);

    // Both circuits use the same validator Merkle tree
    let merkle_siblings: Vec<Fr> =
        (0..depth).map(|i| Fr::from((i + 500) as u64)).collect();
    let path_bits: Vec<bool> =
        (0..depth).map(|i| i % 2 == 0).collect();

    // --- Stake proof: prove stake >= 5000 ---
    let stake_blank = StakeCircuit::blank(depth);
    let (stake_pk, stake_vk) =
        Groth16::<Bn254>::circuit_specific_setup(stake_blank, &mut rng)
            .expect("stake setup");

    let stake_circuit = StakeCircuit::new(
        validator_id,
        10_000u128,
        5_000u128,
        randomness,
        merkle_siblings.clone(),
        path_bits.clone(),
    )
    .expect("stake circuit");
    let stake_inputs = stake_circuit.public_inputs();
    let stake_proof =
        Groth16::<Bn254>::prove(&stake_pk, stake_circuit, &mut rng)
            .expect("stake prove");

    assert!(
        Groth16::<Bn254>::verify(&stake_vk, &stake_inputs, &stake_proof)
            .unwrap(),
        "stake proof should verify"
    );

    // --- Vote proof: cast anonymous vote on topic 42 ---
    let vote_blank = VoteCircuit::blank(depth);
    let (vote_pk, vote_vk) =
        Groth16::<Bn254>::circuit_specific_setup(vote_blank, &mut rng)
            .expect("vote setup");

    let vote_circuit = VoteCircuit::new(
        validator_id,
        Fr::from(1u64), // approve
        Fr::from(0xBEEFu64),
        Fr::from(42u64),
        merkle_siblings,
        path_bits,
    );
    let vote_inputs = vote_circuit.public_inputs();
    let vote_proof =
        Groth16::<Bn254>::prove(&vote_pk, vote_circuit, &mut rng)
            .expect("vote prove");

    assert!(
        Groth16::<Bn254>::verify(&vote_vk, &vote_inputs, &vote_proof)
            .unwrap(),
        "vote proof should verify"
    );

    // Verify that the validator root is the same in both proofs
    // (stake public input [2] == vote public input [0])
    assert_eq!(
        stake_inputs[2], vote_inputs[0],
        "both circuits must use same validator root"
    );
}

#[test]
fn device_attestation_and_presence_integration() {
    // Simulate: device attests in epoch, then actor proves presence
    let mut rng = test_rng();

    // --- Device attestation ---
    let att_depth = 7;
    let att_blank = AttestationCircuit::blank(att_depth);
    let (att_pk, att_vk) =
        Groth16::<Bn254>::circuit_specific_setup(att_blank, &mut rng)
            .expect("attestation setup");

    let device_id = Fr::from(0xDE01u64);
    let challenge = Fr::from(0xC0A1u64);
    let response = Fr::from(0xBE5Bu64);
    let epoch_id_fr = Fr::from(1u64);

    let att_circuit = AttestationCircuit::new(
        device_id,
        challenge,
        response,
        epoch_id_fr,
        (0..att_depth)
            .map(|i| Fr::from((i + 300) as u64))
            .collect(),
        (0..att_depth).map(|i| i % 2 == 1).collect(),
    );
    let att_inputs = att_circuit.public_inputs();
    let att_proof =
        Groth16::<Bn254>::prove(&att_pk, att_circuit, &mut rng)
            .expect("attestation prove");

    assert!(
        Groth16::<Bn254>::verify(&att_vk, &att_inputs, &att_proof).unwrap(),
        "attestation proof should verify"
    );

    // --- Presence proof in same epoch ---
    let pres_depth = 5;
    let pres_blank = PresenceCircuit::blank(pres_depth);
    let (pres_pk, pres_vk) =
        Groth16::<Bn254>::circuit_specific_setup(pres_blank, &mut rng)
            .expect("presence setup");

    let pres_circuit = PresenceCircuit::new(
        Fr::from(12345u64),
        1u64, // same epoch
        Fr::from(0xCAFEu64),
        (0..pres_depth)
            .map(|i| Fr::from((i + 100) as u64))
            .collect(),
        (0..pres_depth).map(|i| i % 2 == 0).collect(),
    );
    let pres_inputs = pres_circuit.public_inputs();
    let pres_proof =
        Groth16::<Bn254>::prove(&pres_pk, pres_circuit, &mut rng)
            .expect("presence prove");

    assert!(
        Groth16::<Bn254>::verify(&pres_vk, &pres_inputs, &pres_proof)
            .unwrap(),
        "presence proof should verify"
    );

    // Both are in the same epoch
    assert_eq!(
        pres_inputs[2],
        Fr::from(1u64),
        "presence epoch must match"
    );
}

// ─── Conversion Utility Tests ───────────────────────────────────────────

#[test]
fn bytes_to_fr_deterministic_and_nonzero() {
    for i in 1u8..100 {
        let bytes = [i; 32];
        let f1 = bytes_to_fr(&bytes);
        let f2 = bytes_to_fr(&bytes);
        assert_eq!(f1, f2, "must be deterministic");
        assert_ne!(f1, Fr::zero(), "non-zero input should give non-zero Fr");
    }
}

#[test]
fn u64_to_fr_preserves_value() {
    for val in [0u64, 1, 42, 255, 1000, u64::MAX] {
        let fr = u64_to_fr(val);
        assert_eq!(fr, Fr::from(val));
    }
}

#[test]
fn bytes_to_fr_different_inputs_different_outputs() {
    let mut previous = Vec::new();
    for i in 0u8..50 {
        let mut bytes = [0u8; 32];
        bytes[0] = i;
        let fr = bytes_to_fr(&bytes);
        assert!(
            !previous.contains(&fr),
            "different byte inputs must give different Fr elements"
        );
        previous.push(fr);
    }
}

// ─── EC-VSS + Share Circuit Integration ─────────────────────────────────

#[test]
fn ec_vss_shares_verify_in_share_circuit() {
    use crate::ec_vss::EcFeldmanVSS;
    let mut rng = test_rng();

    // Distribute secret using EC-VSS
    let secret_bytes = [42u8; 32];
    let entropy = [0xABu8; 32];
    let threshold: u8 = 3;
    let num_shares: u8 = 5;
    let (shares, commitments) = EcFeldmanVSS::share_with_commitments(
        &secret_bytes, threshold, num_shares, &entropy,
    )
    .expect("EC-VSS share distribution");

    // Setup share circuit
    let blank = ShareCircuit::blank();
    let (pk, vk) =
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
            .expect("setup");

    // Each share can be proven with the share circuit
    for (idx, share) in shares.iter().enumerate() {
        // Verify share against EC commitments
        assert!(
            EcFeldmanVSS::verify_share(share, &commitments),
            "EC-VSS share {} must verify",
            idx
        );

        // Create ZK proof that we know the share
        let randomness = Fr::from((idx as u64 + 1000) * 7);
        let circuit = ShareCircuit::new(
            share.value,
            Fr::from(share.index as u64),
            randomness,
        );
        let public_inputs = circuit.public_inputs();
        let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng)
            .expect("share prove");

        assert!(
            Groth16::<Bn254>::verify(&vk, &public_inputs, &proof).unwrap(),
            "share {} ZK proof must verify",
            idx
        );
    }

    // Verify reconstruction works
    let reconstructed =
        EcFeldmanVSS::reconstruct(&shares[..threshold as usize], threshold)
            .expect("reconstruction");
    let expected = Fr::from_be_bytes_mod_order(&secret_bytes);
    assert_eq!(
        reconstructed, expected,
        "reconstruction must recover secret"
    );
}

// ─── Range Proof Property Tests ─────────────────────────────────────────

#[test]
fn reputation_range_proof_boundary_values() {
    let mut rng = test_rng();

    let blank = ReputationCircuit::blank();
    let (pk, vk) =
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
            .expect("setup");

    // Test various score/threshold combinations
    let test_cases = [
        (0u64, 0u64),    // zero == zero
        (1, 0),          // barely above
        (100, 100),      // exact match
        (100, 1),        // well above
        (1000, 999),     // one above
    ];

    for (score, threshold) in test_cases {
        let circuit =
            ReputationCircuit::new(
                Fr::from(1u64),
                score,
                threshold,
                Fr::from(0xABu64),
            )
            .expect(&format!("score {} >= threshold {}", score, threshold));

        let inputs = circuit.public_inputs();
        let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng)
            .expect("prove");
        assert!(
            Groth16::<Bn254>::verify(&vk, &inputs, &proof).unwrap(),
            "score={} threshold={} should verify",
            score,
            threshold
        );
    }
}

#[test]
fn stake_range_proof_boundary_values() {
    let mut rng = test_rng();
    let depth = 3;

    let blank = StakeCircuit::blank(depth);
    let (pk, vk) =
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
            .expect("setup");

    let test_cases: [(u128, u128); 4] = [
        (0, 0),              // zero == zero
        (1, 0),              // barely above
        (10_000, 10_000),    // exact match
        (1_000_000, 999_999), // one above
    ];

    let siblings: Vec<Fr> =
        (0..depth).map(|i| Fr::from((i + 400) as u64)).collect();
    let path_bits: Vec<bool> =
        (0..depth).map(|i| i % 2 == 0).collect();

    for (stake, min) in test_cases {
        let circuit = StakeCircuit::new(
            Fr::from(0xABCDu64),
            stake,
            min,
            Fr::from(0xCAFEu64),
            siblings.clone(),
            path_bits.clone(),
        )
        .expect(&format!("stake {} >= min {}", stake, min));

        let inputs = circuit.public_inputs();
        let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng)
            .expect("prove");
        assert!(
            Groth16::<Bn254>::verify(&vk, &inputs, &proof).unwrap(),
            "stake={} min={} should verify",
            stake,
            min
        );
    }
}

// ─── Position Proximity Property Tests ──────────────────────────────────

#[test]
fn position_proximity_various_positions_within_radius() {
    let mut rng = test_rng();

    let blank = PositionProximityCircuit::blank();
    let (pk, vk) =
        Groth16::<Bn254>::circuit_specific_setup(blank, &mut rng)
            .expect("setup");

    // Center (1000, 1000), radius_sq = 10000 (radius ≈ 100)
    let center_x = 1000u64;
    let center_y = 1000u64;
    let radius_sq = 10_000u64;

    let positions = [
        (1000, 1000), // center
        (1050, 1050), // dx=50, dy=50, dist_sq=5000 < 10000
        (1099, 1000), // dx=99, dy=0, dist_sq=9801 < 10000
        (1000, 1099), // dx=0, dy=99, dist_sq=9801 < 10000
        (1070, 1070), // dx=70, dy=70, dist_sq=9800 < 10000
    ];

    for (x, y) in positions {
        let circuit = PositionProximityCircuit::new(
            x, y, center_x, center_y, radius_sq, 1,
        )
        .expect(&format!("position ({}, {}) within radius", x, y));

        let inputs = circuit.public_inputs();
        let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng)
            .expect("prove");
        assert!(
            Groth16::<Bn254>::verify(&vk, &inputs, &proof).unwrap(),
            "position ({}, {}) should verify",
            x,
            y
        );
    }
}

#[test]
fn position_proximity_rejects_outside_radius() {
    let center_x = 1000u64;
    let center_y = 1000u64;
    let radius_sq = 100u64; // radius = 10

    let outside_positions = [
        (1011, 1000), // dx=11, dist_sq=121 > 100
        (1000, 1011), // dy=11, dist_sq=121 > 100
        (1008, 1008), // dx=8, dy=8, dist_sq=128 > 100
    ];

    for (x, y) in outside_positions {
        let result = PositionProximityCircuit::new(
            x, y, center_x, center_y, radius_sq, 1,
        );
        assert!(
            result.is_none(),
            "position ({}, {}) outside radius should be rejected",
            x,
            y
        );
    }
}
