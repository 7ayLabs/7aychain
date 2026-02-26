//! Stake range proof circuit (zkStake).
//!
//! Proves that a validator's stake is at or above a minimum requirement
//! without revealing the exact stake amount.
//!
//! # Public Inputs
//! - `stake_commitment`: MiMC(validator_id, stake_amount, randomness)
//! - `min_stake`: the minimum stake requirement
//! - `validator_root`: Merkle root of active validator set
//!
//! # Private Witnesses
//! - `validator_id`: the validator's identity (hidden from casual observers)
//! - `stake_amount`: the actual stake (hidden)
//! - `randomness`: blinding factor
//! - `merkle_path`: proof of validator set membership
//! - `path_indices`: left/right bits for the Merkle path
//!
//! # Constraints
//! ~10,500 (validator membership + stake commitment + 128-bit range check)

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{
    merkle_root_native, merkle_verify_gadget, mimc_constants, mimc_hash,
    mimc_hash_gadget,
};

/// Number of bits for the stake range check.
/// 128 bits supports stakes up to 2^128 - 1 smallest units.
pub const STAKE_RANGE_BITS: usize = 128;

/// Default validator set Merkle depth for stake proofs.
pub const DEFAULT_STAKE_DEPTH: usize = 12;

/// R1CS circuit for stake range proof.
#[derive(Clone)]
pub struct StakeCircuit {
    /// Public: MiMC(validator_id, stake_amount, randomness)
    pub stake_commitment: Option<Fr>,
    /// Public: minimum stake requirement
    pub min_stake: Option<Fr>,
    /// Public: Merkle root of validator set
    pub validator_root: Option<Fr>,
    /// Private: validator identity
    pub validator_id: Option<Fr>,
    /// Private: actual stake amount
    pub stake_amount: Option<Fr>,
    /// Private: blinding randomness
    pub randomness: Option<Fr>,
    /// Private: Merkle path siblings
    pub merkle_path: Vec<Option<Fr>>,
    /// Private: Merkle path direction bits
    pub path_indices: Vec<Option<bool>>,
    /// Private: bit decomposition of (stake - min_stake)
    pub diff_bits: Vec<Option<bool>>,
    /// Validator set Merkle depth
    pub depth: usize,
}

impl StakeCircuit {
    /// Create a circuit instance.
    ///
    /// `stake_amount` must be >= `min_stake` (both as u128).
    pub fn new(
        validator_id: Fr,
        stake_amount: u128,
        min_stake: u128,
        randomness: Fr,
        merkle_siblings: Vec<Fr>,
        path_bits: Vec<bool>,
    ) -> Option<Self> {
        if stake_amount < min_stake {
            return None;
        }

        let depth = merkle_siblings.len();
        let diff = stake_amount - min_stake;
        let diff_bits: Vec<bool> = (0..STAKE_RANGE_BITS)
            .map(|i| (diff >> i) & 1 == 1)
            .collect();

        let stake_fr = Fr::from(stake_amount);
        let min_stake_fr = Fr::from(min_stake);

        // Stake commitment = MiMC(validator_id, stake_amount, randomness)
        let stake_commitment =
            mimc_hash(&[validator_id, stake_fr, randomness]);

        // Validator leaf = MiMC(validator_id)
        let leaf = mimc_hash(&[validator_id]);
        let validator_root =
            merkle_root_native(leaf, &merkle_siblings, &path_bits);

        Some(Self {
            stake_commitment: Some(stake_commitment),
            min_stake: Some(min_stake_fr),
            validator_root: Some(validator_root),
            validator_id: Some(validator_id),
            stake_amount: Some(stake_fr),
            randomness: Some(randomness),
            merkle_path: merkle_siblings.into_iter().map(Some).collect(),
            path_indices: path_bits.into_iter().map(Some).collect(),
            diff_bits: diff_bits.into_iter().map(Some).collect(),
            depth,
        })
    }

    /// Create a blank circuit for trusted setup.
    pub fn blank(depth: usize) -> Self {
        Self {
            stake_commitment: Some(Fr::zero()),
            min_stake: Some(Fr::zero()),
            validator_root: Some(Fr::zero()),
            validator_id: Some(Fr::zero()),
            stake_amount: Some(Fr::zero()),
            randomness: Some(Fr::zero()),
            merkle_path: vec![Some(Fr::zero()); depth],
            path_indices: vec![Some(false); depth],
            diff_bits: vec![Some(false); STAKE_RANGE_BITS],
            depth,
        }
    }

    /// Compute the expected public inputs.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![
            self.stake_commitment.unwrap_or(Fr::zero()),
            self.min_stake.unwrap_or(Fr::zero()),
            self.validator_root.unwrap_or(Fr::zero()),
        ]
    }
}

impl ConstraintSynthesizer<Fr> for StakeCircuit {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<Fr>,
    ) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public inputs
        let commitment_var = FpVar::new_input(cs.clone(), || {
            self.stake_commitment
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let min_stake_var = FpVar::new_input(cs.clone(), || {
            self.min_stake.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let validator_root_var = FpVar::new_input(cs.clone(), || {
            self.validator_root
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witnesses
        let validator_id_var = FpVar::new_witness(cs.clone(), || {
            self.validator_id
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let stake_amount_var = FpVar::new_witness(cs.clone(), || {
            self.stake_amount
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let randomness_var = FpVar::new_witness(cs.clone(), || {
            self.randomness.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Allocate Merkle path
        let mut siblings = Vec::with_capacity(self.depth);
        let mut path_bits_var = Vec::with_capacity(self.depth);
        for i in 0..self.depth {
            siblings.push(FpVar::new_witness(cs.clone(), || {
                self.merkle_path
                    .get(i)
                    .and_then(|s| *s)
                    .ok_or(SynthesisError::AssignmentMissing)
            })?);
            path_bits_var.push(Boolean::new_witness(cs.clone(), || {
                self.path_indices
                    .get(i)
                    .and_then(|b| *b)
                    .ok_or(SynthesisError::AssignmentMissing)
            })?);
        }

        // Constraint 1: stake_commitment = MiMC(validator_id, stake, randomness)
        let computed_commitment = mimc_hash_gadget(
            &[
                validator_id_var.clone(),
                stake_amount_var.clone(),
                randomness_var,
            ],
            &constants,
        )?;
        computed_commitment.enforce_equal(&commitment_var)?;

        // Constraint 2: validator Merkle membership
        let leaf =
            mimc_hash_gadget(&[validator_id_var], &constants)?;
        let computed_root = merkle_verify_gadget(
            &leaf,
            &siblings,
            &path_bits_var,
            &constants,
        )?;
        computed_root.enforce_equal(&validator_root_var)?;

        // Constraint 3: range check — prove stake >= min_stake
        let diff = &stake_amount_var - &min_stake_var;

        let mut reconstructed = FpVar::zero();
        let mut power_of_two = FpVar::one();
        let two = FpVar::constant(Fr::from(2u64));

        for i in 0..STAKE_RANGE_BITS {
            let bit = Boolean::new_witness(cs.clone(), || {
                self.diff_bits
                    .get(i)
                    .and_then(|b| *b)
                    .ok_or(SynthesisError::AssignmentMissing)
            })?;
            let bit_fp = FpVar::from(bit);
            reconstructed += &bit_fp * &power_of_two;
            power_of_two *= &two;
        }

        reconstructed.enforce_equal(&diff)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    #[test]
    fn stake_circuit_satisfied() {
        let circuit = StakeCircuit::new(
            Fr::from(0xABCDu64),
            10_000u128,
            5_000u128,
            Fr::from(0xCAFEu64),
            (0..5).map(|i| Fr::from((i + 400) as u64)).collect(),
            (0..5).map(|i| i % 2 == 0).collect(),
        )
        .expect("stake >= min_stake");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn stake_circuit_satisfied_at_minimum() {
        let circuit = StakeCircuit::new(
            Fr::from(0xABCDu64),
            5_000u128,
            5_000u128,
            Fr::from(0xCAFEu64),
            (0..5).map(|i| Fr::from((i + 400) as u64)).collect(),
            (0..5).map(|i| i % 2 == 0).collect(),
        )
        .expect("stake == min_stake");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn stake_circuit_rejects_below_minimum() {
        let result = StakeCircuit::new(
            Fr::from(0xABCDu64),
            4_999u128,
            5_000u128,
            Fr::from(0xCAFEu64),
            (0..5).map(|i| Fr::from((i + 400) as u64)).collect(),
            (0..5).map(|i| i % 2 == 0).collect(),
        );
        assert!(result.is_none());
    }

    #[test]
    fn stake_circuit_constraint_count() {
        let circuit = StakeCircuit::blank(DEFAULT_STAKE_DEPTH);
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let num = cs.num_constraints();
        // Commitment (3 absorptions): ~966
        // Leaf hash + Merkle (12 levels): 322 + 7728 = ~8050
        // Range check (128 bits): ~257
        // Total ≈ 9000-11000
        assert!(
            num > 8500 && num < 12000,
            "unexpected constraint count: {}",
            num
        );
    }
}
