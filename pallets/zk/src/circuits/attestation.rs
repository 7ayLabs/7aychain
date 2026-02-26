//! Device attestation circuit (zkAttest).
//!
//! Proves ownership of a device without exposing the hardware identity or
//! device public key on-chain.
//!
//! # Public Inputs
//! - `device_root`: Merkle root of registered device set
//! - `attestation_commitment`: MiMC(device_id, challenge, response)
//! - `device_nullifier`: MiMC(device_id, epoch_id) — prevents double-attestation
//!
//! # Private Witnesses
//! - `device_id`: the device's unique identifier (hidden)
//! - `challenge`: attestation challenge value
//! - `response`: device's response to the challenge
//! - `epoch_id`: current epoch for nullifier binding
//! - `merkle_path`: proof of device set membership
//! - `path_indices`: left/right bits for the Merkle path
//!
//! # Constraints
//! ~13,000 (device membership + attestation commitment + nullifier)

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{
    merkle_root_native, merkle_verify_gadget, mimc_constants, mimc_hash,
    mimc_hash_gadget,
};

/// Default device set Merkle depth (supports up to 16384 devices).
pub const DEFAULT_DEVICE_DEPTH: usize = 14;

/// R1CS circuit for anonymous device attestation.
#[derive(Clone)]
pub struct AttestationCircuit {
    /// Public: Merkle root of registered device set
    pub device_root: Option<Fr>,
    /// Public: MiMC(device_id, challenge, response)
    pub attestation_commitment: Option<Fr>,
    /// Public: MiMC(device_id, epoch_id) — double-attestation prevention
    pub device_nullifier: Option<Fr>,
    /// Private: device identity
    pub device_id: Option<Fr>,
    /// Private: attestation challenge
    pub challenge: Option<Fr>,
    /// Private: device response
    pub response: Option<Fr>,
    /// Private: epoch identifier
    pub epoch_id: Option<Fr>,
    /// Private: Merkle path siblings
    pub merkle_path: Vec<Option<Fr>>,
    /// Private: Merkle path direction bits
    pub path_indices: Vec<Option<bool>>,
    /// Device set Merkle depth
    pub depth: usize,
}

impl AttestationCircuit {
    /// Create a circuit instance with known witness values.
    pub fn new(
        device_id: Fr,
        challenge: Fr,
        response: Fr,
        epoch_id: Fr,
        merkle_siblings: Vec<Fr>,
        path_bits: Vec<bool>,
    ) -> Self {
        let depth = merkle_siblings.len();

        // Device leaf = MiMC(device_id)
        let leaf = mimc_hash(&[device_id]);
        let device_root =
            merkle_root_native(leaf, &merkle_siblings, &path_bits);

        // Attestation commitment = MiMC(device_id, challenge, response)
        let attestation_commitment =
            mimc_hash(&[device_id, challenge, response]);

        // Device nullifier = MiMC(device_id, epoch_id)
        let device_nullifier = mimc_hash(&[device_id, epoch_id]);

        Self {
            device_root: Some(device_root),
            attestation_commitment: Some(attestation_commitment),
            device_nullifier: Some(device_nullifier),
            device_id: Some(device_id),
            challenge: Some(challenge),
            response: Some(response),
            epoch_id: Some(epoch_id),
            merkle_path: merkle_siblings.into_iter().map(Some).collect(),
            path_indices: path_bits.into_iter().map(Some).collect(),
            depth,
        }
    }

    /// Create a blank circuit for trusted setup.
    pub fn blank(depth: usize) -> Self {
        Self {
            device_root: Some(Fr::zero()),
            attestation_commitment: Some(Fr::zero()),
            device_nullifier: Some(Fr::zero()),
            device_id: Some(Fr::zero()),
            challenge: Some(Fr::zero()),
            response: Some(Fr::zero()),
            epoch_id: Some(Fr::zero()),
            merkle_path: vec![Some(Fr::zero()); depth],
            path_indices: vec![Some(false); depth],
            depth,
        }
    }

    /// Compute the expected public inputs for verification.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![
            self.device_root.unwrap_or(Fr::zero()),
            self.attestation_commitment.unwrap_or(Fr::zero()),
            self.device_nullifier.unwrap_or(Fr::zero()),
        ]
    }
}

impl ConstraintSynthesizer<Fr> for AttestationCircuit {
    fn generate_constraints(
        self,
        cs: ConstraintSystemRef<Fr>,
    ) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public inputs
        let device_root_var = FpVar::new_input(cs.clone(), || {
            self.device_root
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let attestation_commitment_var = FpVar::new_input(cs.clone(), || {
            self.attestation_commitment
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let device_nullifier_var = FpVar::new_input(cs.clone(), || {
            self.device_nullifier
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witnesses
        let device_id_var = FpVar::new_witness(cs.clone(), || {
            self.device_id.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let challenge_var = FpVar::new_witness(cs.clone(), || {
            self.challenge.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let response_var = FpVar::new_witness(cs.clone(), || {
            self.response.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let epoch_id_var = FpVar::new_witness(cs.clone(), || {
            self.epoch_id.ok_or(SynthesisError::AssignmentMissing)
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

        // Constraint 1: device leaf + Merkle membership
        let leaf =
            mimc_hash_gadget(&[device_id_var.clone()], &constants)?;
        let computed_root =
            merkle_verify_gadget(&leaf, &siblings, &path_bits, &constants)?;
        computed_root.enforce_equal(&device_root_var)?;

        // Constraint 2: attestation_commitment = MiMC(device_id, challenge, response)
        let computed_att = mimc_hash_gadget(
            &[device_id_var.clone(), challenge_var, response_var],
            &constants,
        )?;
        computed_att.enforce_equal(&attestation_commitment_var)?;

        // Constraint 3: device_nullifier = MiMC(device_id, epoch_id)
        let computed_null =
            mimc_hash_gadget(&[device_id_var, epoch_id_var], &constants)?;
        computed_null.enforce_equal(&device_nullifier_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    #[test]
    fn attestation_circuit_satisfied() {
        let circuit = AttestationCircuit::new(
            Fr::from(0xDE01u64),
            Fr::from(0xC0A1u64),
            Fr::from(0xBE5Bu64),
            Fr::from(1u64),
            (0..7).map(|i| Fr::from((i + 300) as u64)).collect(),
            (0..7).map(|i| i % 2 == 1).collect(),
        );

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn attestation_circuit_rejects_wrong_device() {
        let mut circuit = AttestationCircuit::new(
            Fr::from(0xDE01u64),
            Fr::from(0xC0A1u64),
            Fr::from(0xBE5Bu64),
            Fr::from(1u64),
            (0..7).map(|i| Fr::from((i + 300) as u64)).collect(),
            (0..7).map(|i| i % 2 == 1).collect(),
        );
        circuit.device_id = Some(Fr::from(0xDE02u64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn attestation_circuit_constraint_count() {
        let circuit = AttestationCircuit::blank(DEFAULT_DEVICE_DEPTH);
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let num = cs.num_constraints();
        assert!(
            num > 10000 && num < 15000,
            "unexpected constraint count: {}",
            num
        );
    }
}
