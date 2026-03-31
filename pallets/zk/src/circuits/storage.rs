//! Storage integrity proof circuit (INV72).
//!
//! Proves that a data hash matches a stored commitment, binding it
//! to a specific epoch and actor without revealing the underlying data.
//!
//! # Public Inputs
//! - `stored_commitment`: MiMC hash of (data, epoch_id, actor_id)
//!
//! # Private Witnesses
//! - `data`: the stored data as Fr element
//! - `epoch_id`: epoch the data is bound to
//! - `actor_id`: actor who owns the data
//!
//! # Constraints
//! ~500 (3 absorptions × 161 rounds × ~1 constraint per cube)

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{mimc_constants, mimc_hash, mimc_hash_gadget};

/// R1CS circuit proving storage data integrity (INV72).
#[derive(Clone)]
pub struct StorageCircuit {
    /// Public: MiMC commitment hash of (data || epoch_id || actor_id)
    pub stored_commitment: Option<Fr>,
    /// Private: the stored data
    pub data: Option<Fr>,
    /// Private: epoch the data is bound to
    pub epoch_id: Option<Fr>,
    /// Private: actor who owns the data
    pub actor_id: Option<Fr>,
}

impl StorageCircuit {
    /// Create a circuit instance with known witness values.
    pub fn new(data: Fr, epoch_id: Fr, actor_id: Fr) -> Self {
        let stored_commitment = mimc_hash(&[data, epoch_id, actor_id]);
        Self {
            stored_commitment: Some(stored_commitment),
            data: Some(data),
            epoch_id: Some(epoch_id),
            actor_id: Some(actor_id),
        }
    }

    /// Create a blank circuit for trusted setup.
    pub fn blank() -> Self {
        Self {
            stored_commitment: Some(Fr::zero()),
            data: Some(Fr::zero()),
            epoch_id: Some(Fr::zero()),
            actor_id: Some(Fr::zero()),
        }
    }

    /// Compute the expected public inputs for verification.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![self.stored_commitment.unwrap_or(Fr::zero())]
    }
}

impl ConstraintSynthesizer<Fr> for StorageCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public input: stored commitment
        let commitment_var = FpVar::new_input(cs.clone(), || {
            self.stored_commitment
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witnesses
        let data_var = FpVar::new_witness(cs.clone(), || {
            self.data.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let epoch_var = FpVar::new_witness(cs.clone(), || {
            self.epoch_id.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let actor_var = FpVar::new_witness(cs, || {
            self.actor_id.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Compute MiMC(data || epoch_id || actor_id) in-circuit
        let computed = mimc_hash_gadget(&[data_var, epoch_var, actor_var], &constants)?;

        // Enforce: computed hash == stored commitment
        computed.enforce_equal(&commitment_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    #[test]
    fn storage_circuit_satisfied_with_valid_witness() {
        let data = Fr::from(0xCAFEu64);
        let epoch_id = Fr::from(5u64);
        let actor_id = Fr::from(0xACu64);

        let circuit = StorageCircuit::new(data, epoch_id, actor_id);
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn storage_circuit_rejects_wrong_commitment() {
        let data = Fr::from(0xCAFEu64);
        let epoch_id = Fr::from(5u64);
        let actor_id = Fr::from(0xACu64);

        let mut circuit = StorageCircuit::new(data, epoch_id, actor_id);
        circuit.stored_commitment = Some(Fr::from(999u64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn storage_circuit_rejects_wrong_data() {
        let data = Fr::from(0xCAFEu64);
        let epoch_id = Fr::from(5u64);
        let actor_id = Fr::from(0xACu64);
        let commitment = mimc_hash(&[data, epoch_id, actor_id]);

        let circuit = StorageCircuit {
            stored_commitment: Some(commitment),
            data: Some(Fr::from(0xBEEFu64)), // wrong data
            epoch_id: Some(epoch_id),
            actor_id: Some(actor_id),
        };

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn storage_circuit_constraint_count() {
        let circuit = StorageCircuit::blank();
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let n = cs.num_constraints();
        assert!(n > 900 && n < 1200, "unexpected constraint count: {}", n);
    }
}
