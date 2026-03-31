//! Key rotation proof circuit (INV78).
//!
//! Proves that a new key is validly derived from an old key via a
//! derivation nonce, without revealing either key value.
//!
//! # Public Inputs
//! - `old_key_hash`: MiMC hash of the old key
//! - `new_key_hash`: MiMC hash of the new key
//!
//! # Private Witnesses
//! - `old_key`: the old key as Fr element
//! - `new_key`: the new key as Fr element
//! - `derivation_nonce`: randomness binding old to new
//!
//! # Constraints
//! ~1500 (3 MiMC evaluations × 161 rounds × ~2 constraints + equalities)

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{mimc_constants, mimc_hash, mimc_hash_gadget};

/// R1CS circuit proving valid key rotation (INV78).
#[derive(Clone)]
pub struct RotationCircuit {
    /// Public: MiMC hash of the old key
    pub old_key_hash: Option<Fr>,
    /// Public: MiMC hash of the new key
    pub new_key_hash: Option<Fr>,
    /// Private: the old key value
    pub old_key: Option<Fr>,
    /// Private: the new key value
    pub new_key: Option<Fr>,
    /// Private: derivation nonce binding old -> new
    pub derivation_nonce: Option<Fr>,
}

impl RotationCircuit {
    /// Create a circuit instance with known witness values.
    ///
    /// Derives `new_key = MiMC(old_key || derivation_nonce)`.
    pub fn new(old_key: Fr, derivation_nonce: Fr) -> Self {
        let old_key_hash = mimc_hash(&[old_key]);
        let new_key = mimc_hash(&[old_key, derivation_nonce]);
        let new_key_hash = mimc_hash(&[new_key]);
        Self {
            old_key_hash: Some(old_key_hash),
            new_key_hash: Some(new_key_hash),
            old_key: Some(old_key),
            new_key: Some(new_key),
            derivation_nonce: Some(derivation_nonce),
        }
    }

    /// Create a blank circuit for trusted setup.
    pub fn blank() -> Self {
        Self {
            old_key_hash: Some(Fr::zero()),
            new_key_hash: Some(Fr::zero()),
            old_key: Some(Fr::zero()),
            new_key: Some(Fr::zero()),
            derivation_nonce: Some(Fr::zero()),
        }
    }

    /// Compute the expected public inputs for verification.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![
            self.old_key_hash.unwrap_or(Fr::zero()),
            self.new_key_hash.unwrap_or(Fr::zero()),
        ]
    }
}

impl ConstraintSynthesizer<Fr> for RotationCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public inputs
        let old_key_hash_var = FpVar::new_input(cs.clone(), || {
            self.old_key_hash.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let new_key_hash_var = FpVar::new_input(cs.clone(), || {
            self.new_key_hash.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witnesses
        let old_key_var = FpVar::new_witness(cs.clone(), || {
            self.old_key.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let new_key_var = FpVar::new_witness(cs.clone(), || {
            self.new_key.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let nonce_var = FpVar::new_witness(cs, || {
            self.derivation_nonce
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Constraint 1: MiMC(old_key) == old_key_hash
        let computed_old_hash =
            mimc_hash_gadget(core::slice::from_ref(&old_key_var), &constants)?;
        computed_old_hash.enforce_equal(&old_key_hash_var)?;

        // Constraint 2: new_key == MiMC(old_key || derivation_nonce)
        let derived_key = mimc_hash_gadget(&[old_key_var, nonce_var], &constants)?;
        derived_key.enforce_equal(&new_key_var)?;

        // Constraint 3: MiMC(new_key) == new_key_hash
        let computed_new_hash = mimc_hash_gadget(&[new_key_var], &constants)?;
        computed_new_hash.enforce_equal(&new_key_hash_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    #[test]
    fn rotation_circuit_satisfied_with_valid_derivation() {
        let old_key = Fr::from(0xDEADu64);
        let nonce = Fr::from(0xBEEFu64);

        let circuit = RotationCircuit::new(old_key, nonce);
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn rotation_circuit_rejects_wrong_old_key_hash() {
        let old_key = Fr::from(0xDEADu64);
        let nonce = Fr::from(0xBEEFu64);

        let mut circuit = RotationCircuit::new(old_key, nonce);
        circuit.old_key_hash = Some(Fr::from(999u64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn rotation_circuit_rejects_wrong_new_key() {
        let old_key = Fr::from(0xDEADu64);
        let nonce = Fr::from(0xBEEFu64);

        let mut circuit = RotationCircuit::new(old_key, nonce);
        // Tamper with new_key to break derivation chain
        circuit.new_key = Some(Fr::from(12345u64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn rotation_circuit_different_nonce_different_key() {
        let old_key = Fr::from(0xDEADu64);
        let c1 = RotationCircuit::new(old_key, Fr::from(1u64));
        let c2 = RotationCircuit::new(old_key, Fr::from(2u64));

        // Same old key hash
        assert_eq!(c1.old_key_hash, c2.old_key_hash);
        // Different new key hash
        assert_ne!(c1.new_key_hash, c2.new_key_hash);
    }

    #[test]
    fn rotation_circuit_constraint_count() {
        let circuit = RotationCircuit::blank();
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let n = cs.num_constraints();
        // 3 MiMC evaluations (old_hash, derivation, new_hash)
        // + 1 MiMC for deriving new_key from (old_key, nonce)
        assert!(n > 1200 && n < 2000, "unexpected constraint count: {}", n);
    }
}
