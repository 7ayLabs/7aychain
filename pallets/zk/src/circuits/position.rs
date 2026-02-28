//! Position proximity proof circuit.
//!
//! Proves that an actor's exact position is within a specified tolerance
//! of a region center, without revealing the exact coordinates.
//!
//! # Public Inputs
//! - `region_commitment`: MiMC(center_x, center_y, radius, epoch_id)
//! - `epoch_id`: the epoch for this proof
//!
//! # Private Witnesses
//! - `exact_x`, `exact_y`: the actor's precise coordinates
//! - `center_x`, `center_y`: the region center
//! - `radius_sq`: squared radius tolerance
//!
//! # Constraints
//! ~1,800 (region commitment hash + distance computation + range check)
//!
//! # Range Check
//!
//! The circuit proves `radius_sq - dist_sq >= 0` by decomposing the difference
//! into `RANGE_BITS` binary digits and verifying each is 0 or 1. This costs
//! `RANGE_BITS` constraints for the bit checks plus 1 for the reconstruction.

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{mimc_constants, mimc_hash, mimc_hash_gadget};

/// Number of bits for the non-negativity range check.
/// 128 bits is sufficient for coordinate distance values.
pub const RANGE_BITS: usize = 128;

/// R1CS circuit for position proximity proof.
#[derive(Clone)]
pub struct PositionProximityCircuit {
    /// Public: MiMC(center_x, center_y, radius_sq, epoch_id)
    pub region_commitment: Option<Fr>,
    /// Public: epoch identifier
    pub epoch_id: Option<Fr>,
    /// Private: exact x-coordinate
    pub exact_x: Option<Fr>,
    /// Private: exact y-coordinate
    pub exact_y: Option<Fr>,
    /// Private: region center x
    pub center_x: Option<Fr>,
    /// Private: region center y
    pub center_y: Option<Fr>,
    /// Private: squared radius tolerance
    pub radius_sq: Option<Fr>,
    /// Private: bit decomposition of (radius_sq - dist_sq)
    /// Pre-computed by the prover for efficiency.
    pub remainder_bits: Vec<Option<bool>>,
}

impl PositionProximityCircuit {
    /// Create a circuit instance with known witness values.
    ///
    /// `exact_x/y` and `center_x/y` are u64 coordinates.
    /// The circuit checks `(x-cx)^2 + (y-cy)^2 <= radius_sq`.
    pub fn new(
        exact_x: u64,
        exact_y: u64,
        center_x: u64,
        center_y: u64,
        radius_sq: u64,
        epoch_id: u64,
    ) -> Option<Self> {
        let dx = exact_x.abs_diff(center_x);
        let dy = exact_y.abs_diff(center_y);

        let dist_sq = dx.checked_mul(dx)?.checked_add(dy.checked_mul(dy)?)?;
        if dist_sq > radius_sq {
            return None; // Position outside radius
        }

        let remainder = radius_sq - dist_sq;
        // Decompose remainder as u128 to support RANGE_BITS > 64
        let remainder_u128 = remainder as u128;
        let remainder_bits: Vec<bool> = (0..RANGE_BITS)
            .map(|i| (remainder_u128 >> i) & 1 == 1)
            .collect();

        let cx_fr = Fr::from(center_x);
        let cy_fr = Fr::from(center_y);
        let rsq_fr = Fr::from(radius_sq);
        let eid_fr = Fr::from(epoch_id);

        let region_commitment = mimc_hash(&[cx_fr, cy_fr, rsq_fr, eid_fr]);

        Some(Self {
            region_commitment: Some(region_commitment),
            epoch_id: Some(eid_fr),
            exact_x: Some(Fr::from(exact_x)),
            exact_y: Some(Fr::from(exact_y)),
            center_x: Some(cx_fr),
            center_y: Some(cy_fr),
            radius_sq: Some(rsq_fr),
            remainder_bits: remainder_bits.into_iter().map(Some).collect(),
        })
    }

    /// Create a blank circuit for trusted setup.
    /// Uses zero values for constraint counting and CRS generation.
    pub fn blank() -> Self {
        Self {
            region_commitment: Some(Fr::zero()),
            epoch_id: Some(Fr::zero()),
            exact_x: Some(Fr::zero()),
            exact_y: Some(Fr::zero()),
            center_x: Some(Fr::zero()),
            center_y: Some(Fr::zero()),
            radius_sq: Some(Fr::zero()),
            remainder_bits: vec![Some(false); RANGE_BITS],
        }
    }

    /// Compute the expected public inputs for verification.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![
            self.region_commitment.unwrap_or(Fr::zero()),
            self.epoch_id.unwrap_or(Fr::zero()),
        ]
    }
}

impl ConstraintSynthesizer<Fr> for PositionProximityCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public inputs
        let region_commitment_var = FpVar::new_input(cs.clone(), || {
            self.region_commitment
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let epoch_id_var = FpVar::new_input(cs.clone(), || {
            self.epoch_id.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witnesses
        let exact_x_var = FpVar::new_witness(cs.clone(), || {
            self.exact_x.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let exact_y_var = FpVar::new_witness(cs.clone(), || {
            self.exact_y.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let center_x_var = FpVar::new_witness(cs.clone(), || {
            self.center_x.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let center_y_var = FpVar::new_witness(cs.clone(), || {
            self.center_y.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let radius_sq_var = FpVar::new_witness(cs.clone(), || {
            self.radius_sq.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Constraint 1: region_commitment = MiMC(center_x, center_y, radius_sq, epoch_id)
        let computed_commitment = mimc_hash_gadget(
            &[
                center_x_var.clone(),
                center_y_var.clone(),
                radius_sq_var.clone(),
                epoch_id_var,
            ],
            &constants,
        )?;
        computed_commitment.enforce_equal(&region_commitment_var)?;

        // Constraint 2: compute distance squared
        let dx = &exact_x_var - &center_x_var;
        let dy = &exact_y_var - &center_y_var;
        let dx_sq = &dx * &dx;
        let dy_sq = &dy * &dy;
        let dist_sq = &dx_sq + &dy_sq;

        // Constraint 3: remainder = radius_sq - dist_sq
        let remainder = &radius_sq_var - &dist_sq;

        // Constraint 4: range check — prove remainder is non-negative
        // by decomposing into RANGE_BITS binary digits
        let mut reconstructed = FpVar::zero();
        let mut power_of_two = FpVar::one();
        let two = FpVar::constant(Fr::from(2u64));

        for i in 0..RANGE_BITS {
            let bit = Boolean::new_witness(cs.clone(), || {
                self.remainder_bits
                    .get(i)
                    .and_then(|b| *b)
                    .ok_or(SynthesisError::AssignmentMissing)
            })?;

            // Add bit * 2^i to reconstructed
            let bit_fp = FpVar::from(bit);
            reconstructed += &bit_fp * &power_of_two;
            power_of_two *= &two;
        }

        // Enforce: reconstructed == remainder
        reconstructed.enforce_equal(&remainder)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    #[test]
    fn position_circuit_satisfied_at_center() {
        let circuit = PositionProximityCircuit::new(100, 100, 100, 100, 2500, 1)
            .expect("position within radius");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn position_circuit_satisfied_at_boundary() {
        // Distance = sqrt(9+16) = 5, radius_sq = 25
        let circuit =
            PositionProximityCircuit::new(103, 104, 100, 100, 25, 1).expect("position at boundary");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn position_circuit_rejects_outside_radius() {
        // Distance = sqrt(100+100) > sqrt(25)
        let result = PositionProximityCircuit::new(110, 110, 100, 100, 25, 1);
        assert!(result.is_none());
    }

    #[test]
    fn position_circuit_rejects_wrong_commitment() {
        let mut circuit = PositionProximityCircuit::new(101, 101, 100, 100, 2500, 1)
            .expect("position within radius");
        circuit.region_commitment = Some(Fr::from(999u64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn position_circuit_constraint_count() {
        let circuit = PositionProximityCircuit::blank();
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let num = cs.num_constraints();
        // MiMC hash (4 absorptions): ~1288
        // Distance: 2 squarings + 1 add = ~3
        // Range check: 128 bits × ~2 + 1 reconstruction = ~257
        // Total ≈ 1500-2000
        assert!(
            num > 1200 && num < 2500,
            "unexpected constraint count: {}",
            num
        );
    }
}
