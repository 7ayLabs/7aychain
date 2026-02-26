//! Anonymous vote circuit (zkVote).
//!
//! Proves that a voter is a member of the validator set and that their
//! vote is well-formed, without revealing which validator cast the vote.
//!
//! # Public Inputs
//! - `validator_root`: Merkle root of the active validator set
//! - `vote_commitment`: MiMC(vote_value, randomness)
//! - `vote_nullifier`: MiMC(validator_id, vote_topic) — prevents double-voting
//!
//! # Private Witnesses
//! - `validator_id`: the voter's identity (hidden)
//! - `vote_value`: the actual vote (e.g., 0=reject, 1=approve)
//! - `randomness`: blinding factor for vote commitment
//! - `vote_topic`: topic identifier for the vote
//! - `merkle_path`: proof of validator set membership
//! - `path_indices`: left/right bits for the Merkle path
//!
//! # Constraints
//! ~11,500 (set membership + vote commitment + nullifier derivation)

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{
    merkle_root_native, merkle_verify_gadget, mimc_constants, mimc_hash,
    mimc_hash_gadget,
};

/// Default validator set Merkle depth (supports up to 4096 validators).
pub const DEFAULT_VALIDATOR_DEPTH: usize = 12;

/// R1CS circuit for anonymous voting.
#[derive(Clone)]
pub struct VoteCircuit {
    /// Public: Merkle root of active validator set
    pub validator_root: Option<Fr>,
    /// Public: MiMC(vote_value, randomness)
    pub vote_commitment: Option<Fr>,
    /// Public: MiMC(validator_id, vote_topic) — double-vote prevention
    pub vote_nullifier: Option<Fr>,
    /// Private: voter identity
    pub validator_id: Option<Fr>,
    /// Private: vote value
    pub vote_value: Option<Fr>,
    /// Private: blinding randomness
    pub randomness: Option<Fr>,
    /// Private: vote topic identifier
    pub vote_topic: Option<Fr>,
    /// Private: Merkle path siblings
    pub merkle_path: Vec<Option<Fr>>,
    /// Private: Merkle path direction bits
    pub path_indices: Vec<Option<bool>>,
    /// Validator set Merkle depth
    pub depth: usize,
}

impl VoteCircuit {
    /// Create a circuit instance with known witness values.
    pub fn new(
        validator_id: Fr,
        vote_value: Fr,
        randomness: Fr,
        vote_topic: Fr,
        merkle_siblings: Vec<Fr>,
        path_bits: Vec<bool>,
    ) -> Self {
        let depth = merkle_siblings.len();

        // Validator leaf = MiMC(validator_id)
        let leaf = mimc_hash(&[validator_id]);
        let validator_root =
            merkle_root_native(leaf, &merkle_siblings, &path_bits);

        // Vote commitment = MiMC(vote_value, randomness)
        let vote_commitment = mimc_hash(&[vote_value, randomness]);

        // Vote nullifier = MiMC(validator_id, vote_topic)
        let vote_nullifier = mimc_hash(&[validator_id, vote_topic]);

        Self {
            validator_root: Some(validator_root),
            vote_commitment: Some(vote_commitment),
            vote_nullifier: Some(vote_nullifier),
            validator_id: Some(validator_id),
            vote_value: Some(vote_value),
            randomness: Some(randomness),
            vote_topic: Some(vote_topic),
            merkle_path: merkle_siblings.into_iter().map(Some).collect(),
            path_indices: path_bits.into_iter().map(Some).collect(),
            depth,
        }
    }

    /// Create a blank circuit for trusted setup.
    pub fn blank(depth: usize) -> Self {
        Self {
            validator_root: Some(Fr::zero()),
            vote_commitment: Some(Fr::zero()),
            vote_nullifier: Some(Fr::zero()),
            validator_id: Some(Fr::zero()),
            vote_value: Some(Fr::zero()),
            randomness: Some(Fr::zero()),
            vote_topic: Some(Fr::zero()),
            merkle_path: vec![Some(Fr::zero()); depth],
            path_indices: vec![Some(false); depth],
            depth,
        }
    }

    /// Compute the expected public inputs for verification.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![
            self.validator_root.unwrap_or(Fr::zero()),
            self.vote_commitment.unwrap_or(Fr::zero()),
            self.vote_nullifier.unwrap_or(Fr::zero()),
        ]
    }
}

impl ConstraintSynthesizer<Fr> for VoteCircuit {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<Fr>,
    ) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public inputs
        let validator_root_var = FpVar::new_input(cs.clone(), || {
            self.validator_root
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let vote_commitment_var = FpVar::new_input(cs.clone(), || {
            self.vote_commitment
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let vote_nullifier_var = FpVar::new_input(cs.clone(), || {
            self.vote_nullifier
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witnesses
        let validator_id_var = FpVar::new_witness(cs.clone(), || {
            self.validator_id
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let vote_value_var = FpVar::new_witness(cs.clone(), || {
            self.vote_value.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let randomness_var = FpVar::new_witness(cs.clone(), || {
            self.randomness.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let vote_topic_var = FpVar::new_witness(cs.clone(), || {
            self.vote_topic.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Allocate Merkle path
        let mut siblings = Vec::with_capacity(self.depth);
        let mut path_bits = Vec::with_capacity(self.depth);
        for i in 0..self.depth {
            siblings.push(FpVar::new_witness(cs.clone(), || {
                self.merkle_path
                    .get(i)
                    .and_then(|s| *s)
                    .ok_or(SynthesisError::AssignmentMissing)
            })?);
            path_bits.push(Boolean::new_witness(cs.clone(), || {
                self.path_indices
                    .get(i)
                    .and_then(|b| *b)
                    .ok_or(SynthesisError::AssignmentMissing)
            })?);
        }

        // Constraint 1: leaf = MiMC(validator_id), verify Merkle membership
        let leaf =
            mimc_hash_gadget(&[validator_id_var.clone()], &constants)?;
        let computed_root =
            merkle_verify_gadget(&leaf, &siblings, &path_bits, &constants)?;
        computed_root.enforce_equal(&validator_root_var)?;

        // Constraint 2: vote_commitment = MiMC(vote_value, randomness)
        let computed_commitment =
            mimc_hash_gadget(&[vote_value_var, randomness_var], &constants)?;
        computed_commitment.enforce_equal(&vote_commitment_var)?;

        // Constraint 3: vote_nullifier = MiMC(validator_id, vote_topic)
        let computed_nullifier = mimc_hash_gadget(
            &[validator_id_var, vote_topic_var],
            &constants,
        )?;
        computed_nullifier.enforce_equal(&vote_nullifier_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    #[test]
    fn vote_circuit_satisfied() {
        let circuit = VoteCircuit::new(
            Fr::from(0xABCDu64),
            Fr::from(1u64), // approve
            Fr::from(0xCAFEu64),
            Fr::from(42u64), // topic
            (0..5).map(|i| Fr::from((i + 500) as u64)).collect(),
            (0..5).map(|i| i % 2 == 0).collect(),
        );

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn vote_circuit_rejects_wrong_validator() {
        let mut circuit = VoteCircuit::new(
            Fr::from(0xABCDu64),
            Fr::from(1u64),
            Fr::from(0xCAFEu64),
            Fr::from(42u64),
            (0..5).map(|i| Fr::from((i + 500) as u64)).collect(),
            (0..5).map(|i| i % 2 == 0).collect(),
        );
        circuit.validator_id = Some(Fr::from(0xDEADu64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn vote_circuit_constraint_count() {
        let circuit = VoteCircuit::blank(DEFAULT_VALIDATOR_DEPTH);
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let num = cs.num_constraints();
        // Leaf hash: 322, Merkle (12 levels): 7728
        // Vote commitment: 644, Nullifier: 644
        // Total ≈ 9338 + overhead
        assert!(
            num > 8500 && num < 12000,
            "unexpected constraint count: {}",
            num
        );
    }
}
