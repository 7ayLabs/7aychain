//! Share proof circuit (INV73).
//!
//! Proves knowledge of `(value, index, randomness)` such that
//! `MiMC(value, index, randomness) == commitment` without revealing the witness.
//!
//! # Public Inputs
//! - `commitment`: MiMC hash of the share data
//!
//! # Private Witnesses
//! - `value`: share value as Fr element
//! - `index`: share index as Fr element
//! - `randomness`: blinding randomness as Fr element
//!
//! # Constraints
//! ~500 (3 absorptions × 161 rounds × ~1 constraint per cube)

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{mimc_constants, mimc_hash, mimc_hash_gadget};

/// R1CS circuit for share proof verification (INV73).
#[derive(Clone)]
pub struct ShareCircuit {
    /// Public: MiMC commitment hash
    pub commitment: Option<Fr>,
    /// Private: share value
    pub value: Option<Fr>,
    /// Private: share index
    pub index: Option<Fr>,
    /// Private: blinding randomness
    pub randomness: Option<Fr>,
}

impl ShareCircuit {
    /// Create a circuit instance with known witness values.
    pub fn new(value: Fr, index: Fr, randomness: Fr) -> Self {
        let commitment = mimc_hash(&[value, index, randomness]);
        Self {
            commitment: Some(commitment),
            value: Some(value),
            index: Some(index),
            randomness: Some(randomness),
        }
    }

    /// Create a blank circuit for trusted setup.
    /// Uses zero values for constraint counting and CRS generation.
    pub fn blank() -> Self {
        Self {
            commitment: Some(Fr::zero()),
            value: Some(Fr::zero()),
            index: Some(Fr::zero()),
            randomness: Some(Fr::zero()),
        }
    }

    /// Compute the expected public inputs for verification.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![self.commitment.unwrap_or(Fr::zero())]
    }
}

impl ConstraintSynthesizer<Fr> for ShareCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public input: commitment
        let commitment_var = FpVar::new_input(cs.clone(), || {
            self.commitment.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witnesses
        let value_var = FpVar::new_witness(cs.clone(), || {
            self.value.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let index_var = FpVar::new_witness(cs.clone(), || {
            self.index.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let randomness_var = FpVar::new_witness(cs, || {
            self.randomness.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Compute MiMC hash in-circuit
        let computed = mimc_hash_gadget(&[value_var, index_var, randomness_var], &constants)?;

        // Enforce equality with public commitment
        computed.enforce_equal(&commitment_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    #[test]
    fn share_circuit_satisfied_with_valid_witness() {
        let value = Fr::from(42u64);
        let index = Fr::from(1u64);
        let randomness = Fr::from(0xDEADBEEFu64);

        let circuit = ShareCircuit::new(value, index, randomness);

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn share_circuit_rejects_wrong_commitment() {
        let value = Fr::from(42u64);
        let index = Fr::from(1u64);
        let randomness = Fr::from(0xDEADBEEFu64);

        let mut circuit = ShareCircuit::new(value, index, randomness);
        circuit.commitment = Some(Fr::from(999u64)); // wrong commitment

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn share_circuit_rejects_wrong_value() {
        let value = Fr::from(42u64);
        let index = Fr::from(1u64);
        let randomness = Fr::from(0xDEADBEEFu64);

        let commitment = mimc_hash(&[value, index, randomness]);

        let circuit = ShareCircuit {
            commitment: Some(commitment),
            value: Some(Fr::from(43u64)), // wrong value
            index: Some(index),
            randomness: Some(randomness),
        };

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn share_circuit_constraint_count() {
        let circuit = ShareCircuit::blank();
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let num_constraints = cs.num_constraints();
        // 3 absorptions × 161 rounds × 2 constraints = 966
        // Plus equality constraint + allocation overhead
        assert!(
            num_constraints > 900 && num_constraints < 1200,
            "unexpected constraint count: {}",
            num_constraints
        );
    }
}
