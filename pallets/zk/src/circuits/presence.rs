//! Presence proof circuit (INV74).
//!
//! Proves that an actor has a valid presence in an epoch by demonstrating:
//! 1. Knowledge of a secret that derives a specific nullifier
//! 2. Merkle membership of a leaf (derived from secret + randomness) in the state tree
//!
//! # Public Inputs
//! - `nullifier`: derived from secret and epoch_id
//! - `state_root`: Merkle root of the presence state tree
//! - `epoch_id`: the epoch being proven
//!
//! # Private Witnesses
//! - `secret`: actor's secret key
//! - `randomness`: blinding factor for the leaf commitment
//! - `merkle_path`: sibling hashes along the Merkle path
//! - `path_indices`: left/right bits for each Merkle level
//!
//! # Constraints
//! ~13,500 (nullifier derivation + leaf hash + 20-level Merkle verification)

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{
    merkle_root_native, merkle_verify_gadget, mimc_constants, mimc_hash,
    mimc_hash_gadget, u64_to_fr, DEFAULT_MERKLE_DEPTH,
};

/// R1CS circuit for presence proof verification (INV74).
#[derive(Clone)]
pub struct PresenceCircuit {
    /// Public: nullifier = MiMC(secret, epoch_id)
    pub nullifier: Option<Fr>,
    /// Public: Merkle root of the presence state tree
    pub state_root: Option<Fr>,
    /// Public: epoch identifier
    pub epoch_id: Option<Fr>,
    /// Private: actor's secret
    pub secret: Option<Fr>,
    /// Private: blinding randomness
    pub randomness: Option<Fr>,
    /// Private: Merkle path siblings
    pub merkle_path: Vec<Option<Fr>>,
    /// Private: Merkle path direction bits (false=left, true=right)
    pub path_indices: Vec<Option<bool>>,
    /// Merkle tree depth
    pub depth: usize,
}

impl PresenceCircuit {
    /// Create a circuit instance with known witness values.
    pub fn new(
        secret: Fr,
        epoch_id: u64,
        randomness: Fr,
        merkle_siblings: Vec<Fr>,
        path_bits: Vec<bool>,
    ) -> Self {
        let depth = merkle_siblings.len();
        let epoch_fr = u64_to_fr(epoch_id);

        // Nullifier = MiMC(secret, epoch_id)
        let nullifier = mimc_hash(&[secret, epoch_fr]);

        // Leaf = MiMC(secret, randomness)
        let leaf = mimc_hash(&[secret, randomness]);

        // Compute Merkle root
        let state_root =
            merkle_root_native(leaf, &merkle_siblings, &path_bits);

        Self {
            nullifier: Some(nullifier),
            state_root: Some(state_root),
            epoch_id: Some(epoch_fr),
            secret: Some(secret),
            randomness: Some(randomness),
            merkle_path: merkle_siblings.into_iter().map(Some).collect(),
            path_indices: path_bits.into_iter().map(Some).collect(),
            depth,
        }
    }

    /// Create a blank circuit for trusted setup.
    /// Uses zero values for constraint counting and CRS generation.
    pub fn blank(depth: usize) -> Self {
        Self {
            nullifier: Some(Fr::zero()),
            state_root: Some(Fr::zero()),
            epoch_id: Some(Fr::zero()),
            secret: Some(Fr::zero()),
            randomness: Some(Fr::zero()),
            merkle_path: vec![Some(Fr::zero()); depth],
            path_indices: vec![Some(false); depth],
            depth,
        }
    }

    /// Create a blank circuit with default depth.
    pub fn blank_default() -> Self {
        Self::blank(DEFAULT_MERKLE_DEPTH)
    }

    /// Compute the expected public inputs for verification.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![
            self.nullifier.unwrap_or(Fr::zero()),
            self.state_root.unwrap_or(Fr::zero()),
            self.epoch_id.unwrap_or(Fr::zero()),
        ]
    }
}

impl ConstraintSynthesizer<Fr> for PresenceCircuit {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<Fr>,
    ) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public inputs
        let nullifier_var = FpVar::new_input(cs.clone(), || {
            self.nullifier.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let state_root_var = FpVar::new_input(cs.clone(), || {
            self.state_root.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let epoch_id_var = FpVar::new_input(cs.clone(), || {
            self.epoch_id.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witnesses
        let secret_var = FpVar::new_witness(cs.clone(), || {
            self.secret.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let randomness_var = FpVar::new_witness(cs.clone(), || {
            self.randomness.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Allocate Merkle path
        let mut siblings = Vec::with_capacity(self.depth);
        let mut path_bits = Vec::with_capacity(self.depth);
        for i in 0..self.depth {
            let sibling = FpVar::new_witness(cs.clone(), || {
                self.merkle_path
                    .get(i)
                    .and_then(|s| *s)
                    .ok_or(SynthesisError::AssignmentMissing)
            })?;
            let bit = Boolean::new_witness(cs.clone(), || {
                self.path_indices
                    .get(i)
                    .and_then(|b| *b)
                    .ok_or(SynthesisError::AssignmentMissing)
            })?;
            siblings.push(sibling);
            path_bits.push(bit);
        }

        // Constraint 1: nullifier = MiMC(secret, epoch_id)
        let computed_nullifier =
            mimc_hash_gadget(&[secret_var.clone(), epoch_id_var], &constants)?;
        computed_nullifier.enforce_equal(&nullifier_var)?;

        // Constraint 2: leaf = MiMC(secret, randomness)
        let leaf =
            mimc_hash_gadget(&[secret_var, randomness_var], &constants)?;

        // Constraint 3: Merkle path from leaf to state_root
        let computed_root =
            merkle_verify_gadget(&leaf, &siblings, &path_bits, &constants)?;
        computed_root.enforce_equal(&state_root_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    fn build_test_circuit(depth: usize) -> PresenceCircuit {
        let secret = Fr::from(12345u64);
        let epoch_id = 1u64;
        let randomness = Fr::from(0xCAFEu64);

        let siblings: Vec<Fr> =
            (0..depth).map(|i| Fr::from((i + 100) as u64)).collect();
        let path_bits: Vec<bool> =
            (0..depth).map(|i| i % 2 == 0).collect();

        PresenceCircuit::new(secret, epoch_id, randomness, siblings, path_bits)
    }

    #[test]
    fn presence_circuit_satisfied() {
        let circuit = build_test_circuit(5);

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn presence_circuit_rejects_wrong_nullifier() {
        let mut circuit = build_test_circuit(5);
        circuit.nullifier = Some(Fr::from(999u64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn presence_circuit_rejects_wrong_root() {
        let mut circuit = build_test_circuit(5);
        circuit.state_root = Some(Fr::from(999u64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn presence_circuit_constraint_count_depth_5() {
        let circuit = PresenceCircuit::blank(5);
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let num = cs.num_constraints();
        // Nullifier: 2 absorptions × 161 × 2 = 644
        // Leaf: 2 absorptions × 161 × 2 = 644
        // Merkle (5 levels): 5 × (2 absorptions × 161 × 2 + select) ≈ 3230
        // Total ≈ 4500-5000
        assert!(
            num > 4000 && num < 6000,
            "unexpected constraint count: {}",
            num
        );
    }

    #[test]
    fn presence_circuit_constraint_count_depth_20() {
        let circuit = PresenceCircuit::blank(DEFAULT_MERKLE_DEPTH);
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let num = cs.num_constraints();
        // Should be ~13000-15000 for depth 20
        assert!(
            num > 12000 && num < 16000,
            "unexpected constraint count at depth 20: {}",
            num
        );
    }
}
