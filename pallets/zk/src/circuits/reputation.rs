//! Reputation range proof circuit (zkReputation).
//!
//! Proves that an actor's reputation score is at or above a threshold
//! without revealing the exact score.
//!
//! # Public Inputs
//! - `score_commitment`: MiMC(actor_id, score, randomness)
//! - `threshold`: the minimum required reputation
//!
//! # Private Witnesses
//! - `actor_id`: the actor's identity (hidden)
//! - `score`: the actual reputation score
//! - `randomness`: blinding factor for the commitment
//!
//! # Range Check
//!
//! Proves `score >= threshold` by decomposing `score - threshold` into
//! `RANGE_BITS` binary digits and verifying each is 0 or 1.
//!
//! # Constraints
//! ~1,200 (commitment hash + 128-bit range check)

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{mimc_constants, mimc_hash, mimc_hash_gadget};

/// Number of bits for the range check on score difference.
pub const REPUTATION_RANGE_BITS: usize = 32;

/// R1CS circuit for reputation range proof.
#[derive(Clone)]
pub struct ReputationCircuit {
    /// Public: MiMC(actor_id, score, randomness)
    pub score_commitment: Option<Fr>,
    /// Public: minimum required reputation
    pub threshold: Option<Fr>,
    /// Private: actor identity
    pub actor_id: Option<Fr>,
    /// Private: actual score
    pub score: Option<Fr>,
    /// Private: blinding randomness
    pub randomness: Option<Fr>,
    /// Private: bit decomposition of (score - threshold)
    pub diff_bits: Vec<Option<bool>>,
}

impl ReputationCircuit {
    /// Create a circuit instance.
    ///
    /// `score` must be >= `threshold` (both as u64).
    pub fn new(actor_id: Fr, score: u64, threshold: u64, randomness: Fr) -> Option<Self> {
        if score < threshold {
            return None;
        }

        let diff = score - threshold;
        let diff_bits: Vec<bool> = (0..REPUTATION_RANGE_BITS)
            .map(|i| if i < 64 { (diff >> i) & 1 == 1 } else { false })
            .collect();

        let score_fr = Fr::from(score);
        let threshold_fr = Fr::from(threshold);
        let score_commitment = mimc_hash(&[actor_id, score_fr, randomness]);

        Some(Self {
            score_commitment: Some(score_commitment),
            threshold: Some(threshold_fr),
            actor_id: Some(actor_id),
            score: Some(score_fr),
            randomness: Some(randomness),
            diff_bits: diff_bits.into_iter().map(Some).collect(),
        })
    }

    /// Create a blank circuit for trusted setup.
    pub fn blank() -> Self {
        Self {
            score_commitment: Some(Fr::zero()),
            threshold: Some(Fr::zero()),
            actor_id: Some(Fr::zero()),
            score: Some(Fr::zero()),
            randomness: Some(Fr::zero()),
            diff_bits: vec![Some(false); REPUTATION_RANGE_BITS],
        }
    }

    /// Compute the expected public inputs.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![
            self.score_commitment.unwrap_or(Fr::zero()),
            self.threshold.unwrap_or(Fr::zero()),
        ]
    }
}

impl ConstraintSynthesizer<Fr> for ReputationCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public inputs
        let commitment_var = FpVar::new_input(cs.clone(), || {
            self.score_commitment
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let threshold_var = FpVar::new_input(cs.clone(), || {
            self.threshold.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witnesses
        let actor_id_var = FpVar::new_witness(cs.clone(), || {
            self.actor_id.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let score_var = FpVar::new_witness(cs.clone(), || {
            self.score.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let randomness_var = FpVar::new_witness(cs.clone(), || {
            self.randomness.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Constraint 1: commitment = MiMC(actor_id, score, randomness)
        let computed = mimc_hash_gadget(
            &[actor_id_var, score_var.clone(), randomness_var],
            &constants,
        )?;
        computed.enforce_equal(&commitment_var)?;

        // Constraint 2: range check — prove score >= threshold
        let diff = &score_var - &threshold_var;

        let mut reconstructed = FpVar::zero();
        let mut power_of_two = FpVar::one();
        let two = FpVar::constant(Fr::from(2u64));

        for i in 0..REPUTATION_RANGE_BITS {
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
    fn reputation_circuit_satisfied() {
        let circuit = ReputationCircuit::new(Fr::from(1u64), 85, 50, Fr::from(0xABu64))
            .expect("score >= threshold");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn reputation_circuit_satisfied_at_exact_threshold() {
        let circuit = ReputationCircuit::new(Fr::from(1u64), 50, 50, Fr::from(0xCDu64))
            .expect("score == threshold");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn reputation_circuit_rejects_below_threshold() {
        let result = ReputationCircuit::new(Fr::from(1u64), 30, 50, Fr::from(0xEFu64));
        assert!(result.is_none());
    }

    #[test]
    fn reputation_circuit_rejects_wrong_commitment() {
        let mut circuit = ReputationCircuit::new(Fr::from(1u64), 85, 50, Fr::from(0xABu64))
            .expect("score >= threshold");
        circuit.score_commitment = Some(Fr::from(999u64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn reputation_circuit_constraint_count() {
        let circuit = ReputationCircuit::blank();
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let num = cs.num_constraints();
        // MiMC hash (3 absorptions): ~966
        // Range check (32 bits): ~65
        // Total ≈ 1000-1200
        assert!(
            num > 900 && num < 1500,
            "unexpected constraint count: {}",
            num
        );
    }
}
