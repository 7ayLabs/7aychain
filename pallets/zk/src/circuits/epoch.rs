//! Epoch state transition proof circuit.
//!
//! Proves a valid epoch lifecycle transition without revealing the
//! transition witness. Enforces the monotonic forward-only state machine:
//! Scheduled(0) -> Active(1) -> Closed(2) -> Finalized(3).
//!
//! # Public Inputs
//! - `old_state`: the current epoch state (0, 1, or 2)
//! - `new_state`: the target epoch state (1, 2, or 3)
//!
//! # Private Witnesses
//! - `transition_witness`: binding witness for the transition
//!
//! # Constraints
//! - `new_state == old_state + 1` (monotonic forward)
//! - `old_state` in {0, 1, 2}: enforced by
//!   `old_state * (old_state - 1) * (old_state - 2) == 0`
//! - `new_state` in {1, 2, 3}: enforced by
//!   `(new_state - 1) * (new_state - 2) * (new_state - 3) == 0`
//!
//! Total: ~10 constraints (very lightweight circuit).

use ark_bn254::Fr;
use ark_ff::Zero;
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};

/// R1CS circuit for epoch state transition proof.
#[derive(Clone)]
pub struct EpochCircuit {
    /// Public: current epoch state (0=Scheduled, 1=Active, 2=Closed)
    pub old_state: Option<Fr>,
    /// Public: target epoch state (1=Active, 2=Closed, 3=Finalized)
    pub new_state: Option<Fr>,
    /// Private: transition binding witness
    pub transition_witness: Option<Fr>,
}

impl EpochCircuit {
    /// Create a circuit instance for a valid state transition.
    ///
    /// `old_state` must be in {0, 1, 2} and `new_state` must equal
    /// `old_state + 1`.
    ///
    /// Returns `None` if the transition is invalid.
    pub fn new(old_state: u64, new_state: u64, transition_witness: Fr) -> Option<Self> {
        // Validate: old_state in {0, 1, 2}
        if old_state > 2 {
            return None;
        }
        // Validate: new_state == old_state + 1
        if new_state != old_state.checked_add(1)? {
            return None;
        }

        Some(Self {
            old_state: Some(Fr::from(old_state)),
            new_state: Some(Fr::from(new_state)),
            transition_witness: Some(transition_witness),
        })
    }

    /// Create a blank circuit for trusted setup.
    pub fn blank() -> Self {
        Self {
            old_state: Some(Fr::zero()),
            new_state: Some(Fr::from(1u64)),
            transition_witness: Some(Fr::zero()),
        }
    }

    /// Compute the expected public inputs for verification.
    pub fn public_inputs(&self) -> Vec<Fr> {
        vec![
            self.old_state.unwrap_or(Fr::zero()),
            self.new_state.unwrap_or(Fr::zero()),
        ]
    }
}

impl ConstraintSynthesizer<Fr> for EpochCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Public inputs
        let old_state_var = FpVar::new_input(cs.clone(), || {
            self.old_state.ok_or(SynthesisError::AssignmentMissing)
        })?;
        let new_state_var = FpVar::new_input(cs.clone(), || {
            self.new_state.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Private witness (for binding, not used in constraints
        // directly but allocated to prevent trivial circuits)
        let _witness_var = FpVar::new_witness(cs, || {
            self.transition_witness
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Constraint 1: new_state == old_state + 1 (monotonic forward)
        let one = FpVar::constant(Fr::from(1u64));
        let expected_new = &old_state_var + &one;
        expected_new.enforce_equal(&new_state_var)?;

        // Constraint 2: old_state in {0, 1, 2}
        // old_state * (old_state - 1) * (old_state - 2) == 0
        let old_minus_1 = &old_state_var - &one;
        let two = FpVar::constant(Fr::from(2u64));
        let old_minus_2 = &old_state_var - &two;
        let product_old = &old_state_var * &old_minus_1;
        let product_old_full = &product_old * &old_minus_2;
        product_old_full.enforce_equal(&FpVar::zero())?;

        // Constraint 3: new_state in {1, 2, 3}
        // (new_state - 1) * (new_state - 2) * (new_state - 3) == 0
        let new_minus_1 = &new_state_var - &one;
        let new_minus_2 = &new_state_var - &two;
        let three = FpVar::constant(Fr::from(3u64));
        let new_minus_3 = &new_state_var - &three;
        let product_new = &new_minus_1 * &new_minus_2;
        let product_new_full = &product_new * &new_minus_3;
        product_new_full.enforce_equal(&FpVar::zero())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_relations::r1cs::ConstraintSystem;

    #[test]
    fn epoch_circuit_scheduled_to_active() {
        let circuit = EpochCircuit::new(0, 1, Fr::from(42u64)).expect("valid transition 0->1");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn epoch_circuit_active_to_closed() {
        let circuit = EpochCircuit::new(1, 2, Fr::from(99u64)).expect("valid transition 1->2");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn epoch_circuit_closed_to_finalized() {
        let circuit = EpochCircuit::new(2, 3, Fr::from(0xCAFEu64)).expect("valid transition 2->3");

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn epoch_circuit_rejects_skip_transition() {
        // Cannot go from Scheduled(0) directly to Closed(2)
        let result = EpochCircuit::new(0, 2, Fr::from(42u64));
        assert!(result.is_none());
    }

    #[test]
    fn epoch_circuit_rejects_backward_transition() {
        // Cannot go from Active(1) back to Scheduled(0)
        let result = EpochCircuit::new(1, 0, Fr::from(42u64));
        assert!(result.is_none());
    }

    #[test]
    fn epoch_circuit_rejects_invalid_old_state() {
        // old_state=3 is not in {0, 1, 2}
        let result = EpochCircuit::new(3, 4, Fr::from(42u64));
        assert!(result.is_none());
    }

    #[test]
    fn epoch_circuit_rejects_same_state() {
        // Cannot stay in the same state
        let result = EpochCircuit::new(1, 1, Fr::from(42u64));
        assert!(result.is_none());
    }

    #[test]
    fn epoch_circuit_rejects_tampered_new_state() {
        // Create a valid circuit then tamper with new_state
        let mut circuit = EpochCircuit::new(0, 1, Fr::from(42u64)).expect("valid transition");
        circuit.new_state = Some(Fr::from(2u64)); // should be 1

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        assert!(!cs.is_satisfied().expect("satisfaction check failed"));
    }

    #[test]
    fn epoch_circuit_constraint_count() {
        let circuit = EpochCircuit::blank();
        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit
            .generate_constraints(cs.clone())
            .expect("constraint generation failed");

        let num = cs.num_constraints();
        // Very lightweight: ~10 constraints
        assert!(num > 3 && num < 30, "unexpected constraint count: {}", num);
    }
}
