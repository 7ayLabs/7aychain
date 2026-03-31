//! Cluster membership proof circuit (INV63).
//!
//! Proves that a subnode belongs to an octopus cluster without revealing
//! the member's identity. Uses a MiMC Merkle tree over cluster members.
//!
//! # Public Inputs
//! - `cluster_root`: MiMC Merkle root of the cluster member set
//!
//! # Private Witnesses
//! - `member_id`: the prover's member identifier as Fr
//! - `siblings`: Merkle sibling hashes along the path
//! - `path_bits`: left/right direction bits for each level
//!
//! # Constraints
//! ~(depth × 2 × 161 × 2 + depth × ~10) for Merkle verification

use alloc::vec::Vec;
use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{merkle_root_native, merkle_verify_gadget, mimc_constants, mimc_hash};

/// Default depth for cluster Merkle trees.
/// Supports up to 2^4 = 16 members (OCTOPUS_MAX_SUBNODES = 8 with margin).
pub const CLUSTER_TREE_DEPTH: usize = 4;

/// R1CS circuit proving cluster membership via Merkle inclusion (INV63).
#[derive(Clone)]
pub struct ClusterCircuit {
    /// Public: Merkle root of the cluster member set
    pub cluster_root: Option<Fr>,
    /// Private: the prover's member identifier
    pub member_id: Option<Fr>,
    /// Private: Merkle sibling hashes along the path
    pub siblings: Vec<Option<Fr>>,
    /// Private: left/right path bits (true = right child)
    pub path_bits: Vec<Option<bool>>,
}

impl ClusterCircuit {
    /// Create a circuit instance with known witness values.
    pub fn new(member_id: Fr, siblings: Vec<Fr>, path_bits: Vec<bool>) -> Self {
        let leaf = mimc_hash(&[member_id]);
        let root = merkle_root_native(leaf, &siblings, &path_bits);
        Self {
            cluster_root: Some(root),
            member_id: Some(member_id),
            siblings: siblings.into_iter().map(Some).collect(),
            path_bits: path_bits.into_iter().map(Some).collect(),
        }
    }

    /// Create a blank circuit for trusted setup at a given depth.
    pub fn blank(depth: usize) -> Self {
        Self {
            cluster_root: Some(Fr::zero()),
            member_id: Some(Fr::zero()),
            siblings: vec![Some(Fr::zero()); depth],
            path_bits: vec![Some(false); depth],
        }
    }

    /// Compute the expected public inputs for verification.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![self.cluster_root.unwrap_or(Fr::zero())]
    }
}

impl ConstraintSynthesizer<Fr> for ClusterCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public input: cluster Merkle root
        let root_var = FpVar::new_input(cs.clone(), || {
            self.cluster_root.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witness: member identifier
        let member_var = FpVar::new_witness(cs.clone(), || {
            self.member_id.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witness: Merkle siblings
        let sibling_vars: Vec<FpVar<Fr>> = self
            .siblings
            .iter()
            .map(|s| FpVar::new_witness(cs.clone(), || s.ok_or(SynthesisError::AssignmentMissing)))
            .collect::<Result<_, _>>()?;

        // Private witness: path direction bits
        let path_bit_vars: Vec<Boolean<Fr>> = self
            .path_bits
            .iter()
            .map(|b| {
                Boolean::new_witness(cs.clone(), || b.ok_or(SynthesisError::AssignmentMissing))
            })
            .collect::<Result<_, _>>()?;

        // Compute leaf = MiMC(member_id)
        let leaf = super::mimc_hash_gadget(&[member_var], &constants)?;

        // Walk up the Merkle tree
        let computed_root = merkle_verify_gadget(&leaf, &sibling_vars, &path_bit_vars, &constants)?;

        // Enforce: computed root == public cluster root
        computed_root.enforce_equal(&root_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    fn build_test_tree() -> (Fr, Fr, Vec<Fr>, Vec<bool>) {
        // Build a depth-2 tree with 4 leaves
        let members = [
            Fr::from(10u64),
            Fr::from(20u64),
            Fr::from(30u64),
            Fr::from(40u64),
        ];
        let leaves: Vec<Fr> = members.iter().map(|m| mimc_hash(&[*m])).collect();

        // Level 1
        let n01 = mimc_hash(&[leaves[0], leaves[1]]);
        let n23 = mimc_hash(&[leaves[2], leaves[3]]);
        // Root
        let root = mimc_hash(&[n01, n23]);

        // Prove membership for member[0] (leftmost leaf)
        // Path: left at level 0, left at level 1
        let siblings = vec![leaves[1], n23];
        let path_bits = vec![false, false];

        (root, members[0], siblings, path_bits)
    }

    #[test]
    fn cluster_circuit_satisfied_with_valid_membership() {
        let (_, member, siblings, path_bits) = build_test_tree();

        let circuit = ClusterCircuit::new(member, siblings, path_bits);
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn cluster_circuit_rejects_non_member() {
        let (root, _, siblings, path_bits) = build_test_tree();

        let fake_member = Fr::from(999u64);
        let circuit = ClusterCircuit {
            cluster_root: Some(root),
            member_id: Some(fake_member),
            siblings: siblings.into_iter().map(Some).collect(),
            path_bits: path_bits.into_iter().map(Some).collect(),
        };

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn cluster_circuit_rejects_wrong_root() {
        let (_, member, siblings, path_bits) = build_test_tree();

        let mut circuit = ClusterCircuit::new(member, siblings, path_bits);
        circuit.cluster_root = Some(Fr::from(0xBADu64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn cluster_circuit_constraint_count() {
        let circuit = ClusterCircuit::blank(CLUSTER_TREE_DEPTH);
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let n = cs.num_constraints();
        // leaf hash + depth levels of Merkle verification
        assert!(n > 1000 && n < 5000, "unexpected constraint count: {}", n);
    }
}
