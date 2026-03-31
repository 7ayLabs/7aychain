//! Distance bounds proof circuit (Triangulation).
//!
//! Proves that a measured distance is within a maximum bound without
//! revealing the exact displacement vector. Used for triangulation-based
//! position verification where only the distance constraint matters.
//!
//! # Public Inputs
//! - `max_distance_sq`: squared maximum allowed distance
//! - `commitment`: MiMC(dx, dy, randomness) binding to the displacement
//!
//! # Private Witnesses
//! - `dx`: x-component of displacement
//! - `dy`: y-component of displacement
//! - `randomness`: blinding factor for the commitment
//!
//! # Constraints
//! ~900 (commitment hash + distance computation + 128-bit range check)
//!
//! # Range Check
//!
//! Proves `max_distance_sq - dist_sq >= 0` by decomposing the difference
//! into `DISTANCE_RANGE_BITS` binary digits.

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

use super::{mimc_constants, mimc_hash, mimc_hash_gadget};

/// Number of bits for the distance range check.
/// 128 bits is sufficient for squared distance values.
pub const DISTANCE_RANGE_BITS: usize = 128;

/// R1CS circuit for distance bounds proof.
#[derive(Clone)]
pub struct DistanceCircuit {
    /// Public: squared maximum allowed distance
    pub max_distance_sq: Option<Fr>,
    /// Public: MiMC(dx, dy, randomness)
    pub commitment: Option<Fr>,
    /// Private: x-component of displacement
    pub dx: Option<Fr>,
    /// Private: y-component of displacement
    pub dy: Option<Fr>,
    /// Private: blinding randomness
    pub randomness: Option<Fr>,
    /// Private: bit decomposition of (max_distance_sq - dist_sq)
    pub remainder_bits: Vec<Option<bool>>,
}

impl DistanceCircuit {
    /// Create a circuit instance with known witness values.
    ///
    /// `dx` and `dy` are signed displacements represented as u64.
    /// The circuit checks `dx^2 + dy^2 <= max_distance_sq`.
    ///
    /// Returns `None` if the distance exceeds the maximum.
    pub fn new(dx: u64, dy: u64, max_distance_sq: u64, randomness: Fr) -> Option<Self> {
        let dist_sq = dx.checked_mul(dx)?.checked_add(dy.checked_mul(dy)?)?;

        if dist_sq > max_distance_sq {
            return None;
        }

        let remainder = max_distance_sq - dist_sq;
        let remainder_u128 = remainder as u128;
        let remainder_bits: Vec<bool> = (0..DISTANCE_RANGE_BITS)
            .map(|i| (remainder_u128 >> i) & 1 == 1)
            .collect();

        let dx_fr = Fr::from(dx);
        let dy_fr = Fr::from(dy);
        let max_sq_fr = Fr::from(max_distance_sq);
        let commitment = mimc_hash(&[dx_fr, dy_fr, randomness]);

        Some(Self {
            max_distance_sq: Some(max_sq_fr),
            commitment: Some(commitment),
            dx: Some(dx_fr),
            dy: Some(dy_fr),
            randomness: Some(randomness),
            remainder_bits: remainder_bits.into_iter().map(Some).collect(),
        })
    }

    /// Create a blank circuit for trusted setup.
    pub fn blank() -> Self {
        Self {
            max_distance_sq: Some(Fr::zero()),
            commitment: Some(Fr::zero()),
            dx: Some(Fr::zero()),
            dy: Some(Fr::zero()),
            randomness: Some(Fr::zero()),
            remainder_bits: vec![Some(false); DISTANCE_RANGE_BITS],
        }
    }

    /// Compute the expected public inputs for verification.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![
            self.max_distance_sq.unwrap_or(Fr::zero()),
            self.commitment.unwrap_or(Fr::zero()),
        ]
    }
}

impl ConstraintSynthesizer<Fr> for DistanceCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        let constants = mimc_constants();

        // Public inputs
        let max_distance_sq_var = FpVar::new_input(cs.clone(), || {
            self.max_distance_sq
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let commitment_var = FpVar::new_input(cs.clone(), || {
            self.commitment.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witnesses
        let dx_var = FpVar::new_witness(cs.clone(), || {
            self.dx.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let dy_var = FpVar::new_witness(cs.clone(), || {
            self.dy.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let randomness_var = FpVar::new_witness(cs.clone(), || {
            self.randomness.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Constraint 1: commitment = MiMC(dx, dy, randomness)
        let computed_commitment = mimc_hash_gadget(
            &[dx_var.clone(), dy_var.clone(), randomness_var],
            &constants,
        )?;
        computed_commitment.enforce_equal(&commitment_var)?;

        // Constraint 2: compute dist_sq = dx * dx + dy * dy
        let dx_sq = &dx_var * &dx_var;
        let dy_sq = &dy_var * &dy_var;
        let dist_sq = &dx_sq + &dy_sq;

        // Constraint 3: remainder = max_distance_sq - dist_sq
        let remainder = &max_distance_sq_var - &dist_sq;

        // Constraint 4: range check -- prove remainder is non-negative
        let mut reconstructed = FpVar::zero();
        let mut power_of_two = FpVar::one();
        let two = FpVar::constant(Fr::from(2u64));

        for i in 0..DISTANCE_RANGE_BITS {
            let bit = Boolean::new_witness(cs.clone(), || {
                self.remainder_bits
                    .get(i)
                    .and_then(|b| *b)
                    .ok_or(SynthesisError::AssignmentMissing)
            })?;
            let bit_fp = FpVar::from(bit);
            reconstructed += &bit_fp * &power_of_two;
            power_of_two *= &two;
        }

        reconstructed.enforce_equal(&remainder)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    #[test]
    fn distance_circuit_satisfied_within_bounds() {
        // dx=3, dy=4, dist_sq=25, max_distance_sq=100
        let circuit =
            DistanceCircuit::new(3, 4, 100, Fr::from(0xABCDu64)).expect("distance within bounds");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn distance_circuit_satisfied_at_exact_boundary() {
        // dx=3, dy=4, dist_sq=25, max_distance_sq=25
        let circuit =
            DistanceCircuit::new(3, 4, 25, Fr::from(0xCAFEu64)).expect("distance at boundary");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn distance_circuit_satisfied_at_zero() {
        // dx=0, dy=0, dist_sq=0, max_distance_sq=100
        let circuit = DistanceCircuit::new(0, 0, 100, Fr::from(0xBEEFu64)).expect("zero distance");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn distance_circuit_rejects_outside_bounds() {
        // dx=10, dy=10, dist_sq=200 > 100
        let result = DistanceCircuit::new(10, 10, 100, Fr::from(0xDEADu64));
        assert!(result.is_none());
    }

    #[test]
    fn distance_circuit_rejects_wrong_commitment() {
        let mut circuit =
            DistanceCircuit::new(3, 4, 100, Fr::from(0xABCDu64)).expect("distance within bounds");
        circuit.commitment = Some(Fr::from(999u64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn distance_circuit_rejects_wrong_max_distance() {
        let mut circuit =
            DistanceCircuit::new(3, 4, 100, Fr::from(0xABCDu64)).expect("distance within bounds");
        // Set max_distance_sq to a value less than dist_sq=25
        // but keep the remainder_bits from the original (which assumed max=100)
        circuit.max_distance_sq = Some(Fr::from(10u64));

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn distance_circuit_constraint_count() {
        let circuit = DistanceCircuit::blank();
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let num = cs.num_constraints();
        // MiMC hash (3 absorptions): ~966
        // Distance: 2 squarings + 1 add = ~3
        // Range check: 128 bits = ~257
        // Total ~1200-1500
        assert!(
            num > 900 && num < 1800,
            "unexpected constraint count: {}",
            num
        );
    }
}
