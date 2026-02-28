//! Access/ring membership proof circuit (INV75).
//!
//! Proves that an actor is a member of a vault's ring without revealing their
//! identity. The circuit demonstrates:
//! 1. Knowledge of `actor_id` and `ring_position` that derive a specific leaf
//! 2. Merkle membership of that leaf in the ring root
//! 3. Correct derivation of an access nullifier from `actor_id` and `vault_id`
//!
//! # Public Inputs
//! - `vault_id`: vault identifier
//! - `ring_root`: Merkle root of the vault's membership ring
//! - `access_nullifier`: prevents double-access within a session
//!
//! # Private Witnesses
//! - `actor_id`: the actor's identity (hidden from verifier)
//! - `ring_position`: position in the ring
//! - `merkle_path`: sibling hashes along the Merkle path
//! - `path_indices`: left/right bits for each level
//!
//! # Constraints
//! ~7,000 (leaf hash + nullifier derivation + 10-level Merkle verification)

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{
    merkle_root_native, merkle_verify_gadget, mimc_constants, mimc_hash, mimc_hash_gadget,
    u64_to_fr,
};

/// Default ring Merkle depth (supports up to 1024 members).
pub const DEFAULT_RING_DEPTH: usize = 10;

/// R1CS circuit for access/ring membership proof (INV75).
#[derive(Clone)]
pub struct AccessCircuit {
    /// Public: vault identifier
    pub vault_id: Option<Fr>,
    /// Public: Merkle root of the vault ring
    pub ring_root: Option<Fr>,
    /// Public: access nullifier = MiMC(actor_id, vault_id)
    pub access_nullifier: Option<Fr>,
    /// Private: actor's identity
    pub actor_id: Option<Fr>,
    /// Private: ring position
    pub ring_position: Option<Fr>,
    /// Private: Merkle path siblings
    pub merkle_path: Vec<Option<Fr>>,
    /// Private: Merkle path direction bits
    pub path_indices: Vec<Option<bool>>,
    /// Ring Merkle tree depth
    pub depth: usize,
}

impl AccessCircuit {
    /// Create a circuit instance with known witness values.
    pub fn new(
        actor_id: Fr,
        vault_id: u64,
        ring_position: Fr,
        merkle_siblings: Vec<Fr>,
        path_bits: Vec<bool>,
    ) -> Self {
        let depth = merkle_siblings.len();
        let vault_id_fr = u64_to_fr(vault_id);

        // Leaf = MiMC(actor_id, ring_position)
        let leaf = mimc_hash(&[actor_id, ring_position]);

        // Ring root from Merkle path
        let ring_root = merkle_root_native(leaf, &merkle_siblings, &path_bits);

        // Access nullifier = MiMC(actor_id, vault_id)
        let access_nullifier = mimc_hash(&[actor_id, vault_id_fr]);

        Self {
            vault_id: Some(vault_id_fr),
            ring_root: Some(ring_root),
            access_nullifier: Some(access_nullifier),
            actor_id: Some(actor_id),
            ring_position: Some(ring_position),
            merkle_path: merkle_siblings.into_iter().map(Some).collect(),
            path_indices: path_bits.into_iter().map(Some).collect(),
            depth,
        }
    }

    /// Create a blank circuit for trusted setup.
    /// Uses zero values for constraint counting and CRS generation.
    pub fn blank(depth: usize) -> Self {
        Self {
            vault_id: Some(Fr::zero()),
            ring_root: Some(Fr::zero()),
            access_nullifier: Some(Fr::zero()),
            actor_id: Some(Fr::zero()),
            ring_position: Some(Fr::zero()),
            merkle_path: vec![Some(Fr::zero()); depth],
            path_indices: vec![Some(false); depth],
            depth,
        }
    }

    /// Create a blank circuit with default ring depth.
    pub fn blank_default() -> Self {
        Self::blank(DEFAULT_RING_DEPTH)
    }

    /// Compute the expected public inputs for verification.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![
            self.vault_id.unwrap_or(Fr::zero()),
            self.ring_root.unwrap_or(Fr::zero()),
            self.access_nullifier.unwrap_or(Fr::zero()),
        ]
    }
}

impl ConstraintSynthesizer<Fr> for AccessCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public inputs
        let vault_id_var = FpVar::new_input(cs.clone(), || {
            self.vault_id.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let ring_root_var = FpVar::new_input(cs.clone(), || {
            self.ring_root.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let access_nullifier_var = FpVar::new_input(cs.clone(), || {
            self.access_nullifier
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witnesses
        let actor_id_var = FpVar::new_witness(cs.clone(), || {
            self.actor_id.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let ring_position_var = FpVar::new_witness(cs.clone(), || {
            self.ring_position.ok_or(SynthesisError::AssignmentMissing)
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

        // Constraint 1: leaf = MiMC(actor_id, ring_position)
        let leaf = mimc_hash_gadget(&[actor_id_var.clone(), ring_position_var], &constants)?;

        // Constraint 2: Merkle path from leaf to ring_root
        let computed_root = merkle_verify_gadget(&leaf, &siblings, &path_bits, &constants)?;
        computed_root.enforce_equal(&ring_root_var)?;

        // Constraint 3: access_nullifier = MiMC(actor_id, vault_id)
        let computed_nullifier = mimc_hash_gadget(&[actor_id_var, vault_id_var], &constants)?;
        computed_nullifier.enforce_equal(&access_nullifier_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    fn build_test_circuit(depth: usize) -> AccessCircuit {
        let actor_id = Fr::from(0xABCDu64);
        let vault_id = 7u64;
        let ring_position = Fr::from(3u64);

        let siblings: Vec<Fr> = (0..depth).map(|i| Fr::from((i + 200) as u64)).collect();
        let path_bits: Vec<bool> = (0..depth).map(|i| i % 3 == 0).collect();

        AccessCircuit::new(actor_id, vault_id, ring_position, siblings, path_bits)
    }

    #[test]
    fn access_circuit_satisfied() {
        let circuit = build_test_circuit(5);

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn access_circuit_rejects_wrong_actor() {
        let mut circuit = build_test_circuit(5);
        circuit.actor_id = Some(Fr::from(0xFFFFu64)); // wrong actor

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn access_circuit_rejects_wrong_ring_root() {
        let mut circuit = build_test_circuit(5);
        circuit.ring_root = Some(Fr::from(999u64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn access_circuit_constraint_count_default() {
        let circuit = AccessCircuit::blank(DEFAULT_RING_DEPTH);
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let num = cs.num_constraints();
        // Leaf: 2 absorptions = 644
        // Nullifier: 2 absorptions = 644
        // Merkle (10 levels): 10 × ~645 = 6450
        // Total ≈ 7000-8000
        assert!(
            num > 6500 && num < 9000,
            "unexpected constraint count: {}",
            num
        );
    }
}
