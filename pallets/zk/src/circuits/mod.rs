//! ZK circuit definitions for the 7ay Proof of Presence protocol.
//!
//! Implements R1CS constraint systems for Groth16 BN254 proving/verification:
//!
//! - [`ShareCircuit`](share::ShareCircuit) (INV73): knowledge of commitment preimage
//! - [`PresenceCircuit`](presence::PresenceCircuit) (INV74): nullifier derivation +
//!   Merkle membership
//! - [`AccessCircuit`](access::AccessCircuit) (INV75): ring membership + access nullifier
//! - [`PositionProximityCircuit`](position::PositionProximityCircuit): distance within
//!   tolerance
//!
//! All circuits use MiMC-3/161 as the ZK-friendly hash function. Off-chain provers
//! generate Groth16 BN254 proofs using these circuits; on-chain verification uses
//! `Groth16Verifier::verify_snark`.
//!
//! # MiMC Hash
//!
//! MiMC-3/161 applies 161 rounds of `x -> (x + c_i)^3` over BN254 Fr. This provides
//! ~128-bit security (`ceil(log_3(p)) = 161` for BN254 scalar field). Each round
//! requires 2 R1CS constraints (one squaring, one multiplication), giving ~322
//! constraints per hash absorption.

pub mod access;
pub mod attestation;
pub mod position;
pub mod presence;
pub mod reputation;
pub mod share;
pub mod stake;
pub mod vote;

pub use access::AccessCircuit;
pub use attestation::AttestationCircuit;
pub use position::PositionProximityCircuit;
pub use presence::PresenceCircuit;
pub use reputation::ReputationCircuit;
pub use share::ShareCircuit;
pub use stake::StakeCircuit;
pub use vote::VoteCircuit;

use ark_bn254::Fr;
use ark_ff::{Field, PrimeField, Zero};
use ark_r1cs_std::fields::fp::FpVar;
use ark_r1cs_std::prelude::*;
use ark_relations::r1cs::SynthesisError;

/// Number of MiMC rounds for BN254 security.
/// `ceil(log_3(p))` where p is the BN254 scalar field order.
pub const MIMC_ROUNDS: usize = 161;

/// Domain separator for MiMC round constant generation.
const MIMC_DOMAIN: &[u8; 15] = b"7ay:mimc:bn254:";

/// Default Merkle tree depth for presence/access circuits.
pub const DEFAULT_MERKLE_DEPTH: usize = 20;

/// Generate MiMC round constants deterministically.
///
/// Each constant is derived as `Fr::from_be_bytes_mod_order(blake2_256(domain || i))`.
pub fn mimc_constants() -> Vec<Fr> {
    (0..MIMC_ROUNDS)
        .map(|i| {
            let mut data = [0u8; 23];
            data[..15].copy_from_slice(MIMC_DOMAIN);
            data[15..23].copy_from_slice(&(i as u64).to_le_bytes());
            Fr::from_be_bytes_mod_order(&sp_core::blake2_256(&data))
        })
        .collect()
}

/// Native MiMC permutation: applies `MIMC_ROUNDS` of `x -> (x + c_i)^3`.
fn mimc_permutation_native(mut state: Fr, constants: &[Fr]) -> Fr {
    for c in constants {
        let t = state + c;
        state = t.square() * t;
    }
    state
}

/// Native MiMC sponge hash over a sequence of field elements.
///
/// Absorbs each input into the state, then applies the MiMC permutation.
pub fn mimc_hash(inputs: &[Fr]) -> Fr {
    let constants = mimc_constants();
    let mut state = Fr::zero();
    for input in inputs {
        state += input;
        state = mimc_permutation_native(state, &constants);
    }
    state
}

/// R1CS gadget for MiMC permutation.
///
/// Each round adds 2 constraints (squaring + multiplication).
fn mimc_permutation_gadget(
    state: &FpVar<Fr>,
    constants: &[Fr],
) -> Result<FpVar<Fr>, SynthesisError> {
    let mut s = state.clone();
    for c in constants {
        let c_var = FpVar::constant(*c);
        let t = &s + &c_var;
        let t2 = &t * &t;
        s = &t2 * &t;
    }
    Ok(s)
}

/// R1CS gadget for MiMC sponge hash.
///
/// Constraint cost: `MIMC_ROUNDS * 2 * inputs.len()`.
pub fn mimc_hash_gadget(
    inputs: &[FpVar<Fr>],
    constants: &[Fr],
) -> Result<FpVar<Fr>, SynthesisError> {
    let mut state = FpVar::zero();
    for input in inputs {
        state = &state + input;
        state = mimc_permutation_gadget(&state, constants)?;
    }
    Ok(state)
}

/// R1CS gadget for Merkle path verification.
///
/// Walks from `leaf` up through `siblings` using `path_bits` to determine
/// left/right ordering at each level. Returns the computed root.
///
/// Constraint cost: `(MIMC_ROUNDS * 2 * 2 + ~10) * depth`.
pub fn merkle_verify_gadget(
    leaf: &FpVar<Fr>,
    siblings: &[FpVar<Fr>],
    path_bits: &[Boolean<Fr>],
    constants: &[Fr],
) -> Result<FpVar<Fr>, SynthesisError> {
    let mut current = leaf.clone();
    for (sibling, bit) in siblings.iter().zip(path_bits.iter()) {
        // bit=0: current is left child; bit=1: current is right child
        let left = FpVar::conditionally_select(bit, sibling, &current)?;
        let right = FpVar::conditionally_select(bit, &current, sibling)?;
        current = mimc_hash_gadget(&[left, right], constants)?;
    }
    Ok(current)
}

/// Native Merkle root computation (for test vector generation).
pub fn merkle_root_native(leaf: Fr, siblings: &[Fr], path_bits: &[bool]) -> Fr {
    let mut current = leaf;
    for (sibling, bit) in siblings.iter().zip(path_bits.iter()) {
        let (left, right) = if *bit {
            (*sibling, current)
        } else {
            (current, *sibling)
        };
        current = mimc_hash(&[left, right]);
    }
    current
}

/// Convert a 32-byte value to a BN254 Fr element via mod reduction.
pub fn bytes_to_fr(bytes: &[u8; 32]) -> Fr {
    Fr::from_be_bytes_mod_order(bytes)
}

/// Convert a u64 to a BN254 Fr element.
pub fn u64_to_fr(val: u64) -> Fr {
    Fr::from(val)
}

/// Convert a u32 to a BN254 Fr element.
pub fn u32_to_fr(val: u32) -> Fr {
    Fr::from(val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mimc_constants_deterministic() {
        let c1 = mimc_constants();
        let c2 = mimc_constants();
        assert_eq!(c1, c2);
        assert_eq!(c1.len(), MIMC_ROUNDS);
    }

    #[test]
    fn mimc_constants_nonzero() {
        let constants = mimc_constants();
        for c in &constants {
            assert_ne!(*c, Fr::zero());
        }
    }

    #[test]
    fn mimc_hash_deterministic() {
        let inputs = vec![Fr::from(1u64), Fr::from(2u64)];
        let h1 = mimc_hash(&inputs);
        let h2 = mimc_hash(&inputs);
        assert_eq!(h1, h2);
    }

    #[test]
    fn mimc_hash_different_inputs() {
        let h1 = mimc_hash(&[Fr::from(1u64)]);
        let h2 = mimc_hash(&[Fr::from(2u64)]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn mimc_hash_empty_is_zero_permutation() {
        let h = mimc_hash(&[]);
        assert_eq!(h, Fr::zero());
    }

    #[test]
    fn merkle_root_single_level() {
        let leaf = Fr::from(42u64);
        let sibling = Fr::from(99u64);

        let root_left = merkle_root_native(leaf, &[sibling], &[false]);
        let root_right = merkle_root_native(leaf, &[sibling], &[true]);

        assert_ne!(root_left, root_right);
        assert_eq!(root_left, mimc_hash(&[leaf, sibling]));
        assert_eq!(root_right, mimc_hash(&[sibling, leaf]));
    }

    #[test]
    fn bytes_to_fr_deterministic() {
        let bytes = [0xABu8; 32];
        let f1 = bytes_to_fr(&bytes);
        let f2 = bytes_to_fr(&bytes);
        assert_eq!(f1, f2);
    }
}
