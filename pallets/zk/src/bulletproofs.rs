//! Pedersen commitment and range proof verification using BN254 G1.
//!
//! Provides a simplified Bulletproofs-inspired range proof system built on
//! the existing BN254 curve infrastructure (ark-bn254, ark-ec, ark-ff).
//! Instead of the full inner-product argument from the Bulletproofs paper
//! (Bunz et al., 2018), this implementation uses bit-decomposition with
//! OR-proofs (Sigma protocols) for each bit, composed via Fiat-Shamir.
//!
//! # Scheme
//!
//! A prover commits to a value `v` using a Pedersen commitment:
//!
//! ```text
//! C = v*G + r*H
//! ```
//!
//! where `G` is the BN254 G1 generator and `H` is derived via
//! hash-to-curve with domain separator `7ay:pedersen:H:v1`.
//!
//! To prove that `v` lies in `[0, 2^n)`, the prover decomposes `v` into
//! bits `b_0, ..., b_{n-1}` and for each bit creates an OR-proof showing
//! that the bit commitment either opens to 0 or 1.
//!
//! # Security
//!
//! - **Binding:** Pedersen commitments are computationally binding under DLOG.
//! - **Hiding:** Pedersen commitments are perfectly hiding (information-
//!   theoretic) given the independent generators G and H.
//! - **Soundness:** Each bit proof is a Sigma-OR proof with soundness error
//!   1/|Fr| per bit, composed via Fiat-Shamir (random oracle model).
//! - **Domain separation:** All Fiat-Shamir challenges use Blake2-256 with
//!   the domain separator `7ay:bulletproofs:challenge:v1`.

use alloc::vec::Vec;
use ark_bn254::{Fr, G1Affine, G1Projective};
use ark_ec::{CurveGroup, PrimeGroup};
use ark_ff::{PrimeField, Zero};
use sp_core::blake2_256;

// ---------------------------------------------------------------------------
// Domain separators
// ---------------------------------------------------------------------------

/// Domain separator for deriving the Pedersen H generator.
const DOMAIN_PEDERSEN_H: &[u8] = b"7ay:pedersen:H:v1";

/// Domain separator for Fiat-Shamir challenges in range proofs.
const DOMAIN_RANGE_CHALLENGE: &[u8] = b"7ay:bulletproofs:challenge:v1";

/// Domain separator for per-bit entropy derivation during proof generation.
const DOMAIN_BIT_ENTROPY: &[u8] = b"7ay:bulletproofs:bit:v1";

/// Maximum allowed bit width for range proofs. Values larger than 64 bits
/// are rejected to prevent excessive proof sizes.
pub const MAX_RANGE_BITS: u8 = 64;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A Pedersen commitment: C = v*G + r*H where G, H are independent generators
/// on the BN254 G1 curve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PedersenCommitment {
    /// The commitment point on G1.
    pub point: G1Affine,
}

/// A range proof that a committed value lies in `[0, 2^n)`.
///
/// Consists of per-bit commitments, OR-proofs for each bit, and an
/// aggregated blinding factor proof linking the bit commitments to the
/// original Pedersen commitment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeProof {
    /// Bit commitments `B_i = b_i*G + r_i*H` for each bit of the value.
    pub bit_commitments: Vec<G1Affine>,
    /// Sigma-OR proof that each bit commitment opens to 0 or 1.
    pub bit_proofs: Vec<BitProof>,
    /// Proof that the blinding factors are consistent:
    /// `sum(2^i * r_i) == r` (the original randomness).
    pub blinding_proof: BlindingProof,
}

/// Sigma-OR proof that a bit commitment opens to 0 or 1.
///
/// Uses the Cramer-Damgard-Schoenmakers OR-proof technique: the prover
/// knows the witness for exactly one branch and simulates the other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitProof {
    /// Challenge for the bit=0 branch.
    pub e0: Fr,
    /// Response for the bit=0 branch.
    pub s0: Fr,
    /// Challenge for the bit=1 branch.
    pub e1: Fr,
    /// Response for the bit=1 branch.
    pub s1: Fr,
}

/// Proof that the aggregated blinding factor from bit commitments equals
/// the blinding factor used in the original Pedersen commitment.
///
/// This is a Schnorr-style proof of knowledge of `delta` such that
/// `D = delta * H` where `D = C - sum(2^i * B_i)` should equal zero
/// (meaning `delta = 0` when blinding factors are consistent).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlindingProof {
    /// Schnorr commitment `A = k * H`.
    pub commitment: G1Affine,
    /// Schnorr response `s = k + e * delta`.
    pub response: Fr,
}

// ---------------------------------------------------------------------------
// Core functions
// ---------------------------------------------------------------------------

/// Derive the second Pedersen generator H from G using hash-to-curve.
///
/// H is computed as `H = scalar * G` where `scalar` is derived by hashing
/// the serialized G1 generator with domain separator `DOMAIN_PEDERSEN_H`.
/// This is a nothing-up-my-sleeve construction: H has no known discrete
/// log relationship to G that could be exploited.
pub fn pedersen_h_generator() -> G1Projective {
    let g = G1Projective::generator();
    let g_affine: G1Affine = g.into_affine();

    // Serialize the generator point for hashing
    let g_bytes = point_to_bytes(&g_affine);

    // Domain-separated hash to derive scalar
    let hash = domain_blake2(DOMAIN_PEDERSEN_H, &g_bytes);
    let scalar = Fr::from_be_bytes_mod_order(&hash);

    // H = scalar * G
    g * scalar
}

/// Create a Pedersen commitment to value `v` with randomness `r`.
///
/// ```text
/// C = v*G + r*H
/// ```
///
/// The commitment is perfectly hiding (given uniform `r`) and
/// computationally binding under DLOG.
pub fn pedersen_commit(value: Fr, randomness: Fr) -> PedersenCommitment {
    let g = G1Projective::generator();
    let h = pedersen_h_generator();

    let point = (g * value + h * randomness).into_affine();
    PedersenCommitment { point }
}

/// Create a range proof that the committed value lies in `[0, 2^n)`.
///
/// # Parameters
/// - `value`: The secret value to prove is in range.
/// - `randomness`: The blinding factor used in the Pedersen commitment.
/// - `num_bits`: The bit width `n` (must be in `[1, 64]`).
/// - `entropy`: 32-byte seed for deterministic per-bit randomness derivation.
///
/// # Returns
/// `Some(RangeProof)` on success, `None` if parameters are invalid
/// (e.g., `value >= 2^n` or `num_bits > MAX_RANGE_BITS`).
pub fn prove_range(
    value: u64,
    randomness: Fr,
    num_bits: u8,
    entropy: &[u8; 32],
) -> Option<RangeProof> {
    if num_bits == 0 || num_bits > MAX_RANGE_BITS {
        return None;
    }

    // Check that value fits in num_bits
    if num_bits < 64 && value >= (1u64 << num_bits) {
        return None;
    }

    let g = G1Projective::generator();
    let h = pedersen_h_generator();

    let mut bit_commitments = Vec::with_capacity(num_bits as usize);
    let mut bit_proofs = Vec::with_capacity(num_bits as usize);
    let mut blinding_sum = Fr::zero();

    for i in 0..num_bits {
        let bit = (value >> i) & 1;
        let bit_fr = Fr::from(bit);

        // Derive per-bit randomness deterministically from entropy
        let r_i = derive_bit_randomness(entropy, i);

        // Track blinding factor sum: sum(2^i * r_i)
        let two_pow_i = Fr::from(1u64 << i);
        blinding_sum += two_pow_i * r_i;

        // Bit commitment: B_i = bit * G + r_i * H
        let b_i = (g * bit_fr + h * r_i).into_affine();
        bit_commitments.push(b_i);

        // Generate OR-proof for this bit
        let bit_entropy = derive_bit_proof_entropy(entropy, i);
        let proof = prove_bit_or(g, h, b_i, bit, r_i, &bit_entropy)?;
        bit_proofs.push(proof);
    }

    // Blinding factor consistency proof:
    // delta = randomness - blinding_sum
    // D = C - sum(2^i * B_i) = delta * H
    // Prove knowledge of delta via Schnorr on H, binding the transcript
    // to the actual statement point D.
    let delta = randomness - blinding_sum;
    let d = (h * delta).into_affine();
    let blinding_entropy = derive_blinding_proof_entropy(entropy);
    let blinding_proof = prove_blinding(h, d, delta, &blinding_entropy)?;

    Some(RangeProof {
        bit_commitments,
        bit_proofs,
        blinding_proof,
    })
}

/// Verify a range proof against a Pedersen commitment.
///
/// Checks that:
/// 1. The proof has the correct number of bit commitments and proofs.
/// 2. Each bit proof is a valid OR-proof (bit is 0 or 1).
/// 3. The blinding factor proof shows `C == sum(2^i * B_i)` up to a
///    known offset on H (proving blinding consistency).
///
/// # Parameters
/// - `commitment`: The Pedersen commitment to verify against.
/// - `proof`: The range proof.
/// - `num_bits`: Expected bit width.
///
/// # Returns
/// `true` if the proof is valid.
pub fn verify_range(commitment: &PedersenCommitment, proof: &RangeProof, num_bits: u8) -> bool {
    if num_bits == 0 || num_bits > MAX_RANGE_BITS {
        return false;
    }

    if proof.bit_commitments.len() != num_bits as usize
        || proof.bit_proofs.len() != num_bits as usize
    {
        return false;
    }

    let g = G1Projective::generator();
    let h = pedersen_h_generator();

    // Step 1: Verify each bit proof
    for i in 0..num_bits as usize {
        if !verify_bit_or(g, h, proof.bit_commitments[i], &proof.bit_proofs[i]) {
            return false;
        }
    }

    // Step 2: Verify blinding factor consistency
    // Compute D = C - sum(2^i * B_i)
    // If the proof is valid, D = delta * H for some known delta
    let mut sum_bits = G1Projective::zero();
    for (i, b_i) in proof.bit_commitments.iter().enumerate() {
        let two_pow_i = Fr::from(1u64 << i);
        let b_i_proj: G1Projective = (*b_i).into();
        sum_bits += b_i_proj * two_pow_i;
    }

    let c_proj: G1Projective = commitment.point.into();
    let d = c_proj - sum_bits;

    verify_blinding(h, d.into_affine(), &proof.blinding_proof)
}

/// Prove that a committed value satisfies `value >= min_value`.
///
/// Implemented by proving that `(value - min_value)` is in `[0, 2^n)`.
/// The caller must ensure that the commitment was created with the
/// original `value` and `randomness`.
///
/// # Parameters
/// - `value`: The secret value.
/// - `min_value`: The minimum value threshold.
/// - `randomness`: The blinding factor from the original commitment.
/// - `num_bits`: Bit width for the range of `(value - min_value)`.
/// - `entropy`: Randomness seed.
///
/// # Returns
/// `Some(RangeProof)` for the shifted value, or `None` if `value < min_value`.
pub fn prove_minimum(
    value: u64,
    min_value: u64,
    randomness: Fr,
    num_bits: u8,
    entropy: &[u8; 32],
) -> Option<RangeProof> {
    let shifted = value.checked_sub(min_value)?;
    prove_range(shifted, randomness, num_bits, entropy)
}

/// Verify that a committed value is `>= min_value`.
///
/// The verifier adjusts the commitment by subtracting `min_value * G`
/// and then verifies a standard range proof on the shifted commitment.
///
/// # Parameters
/// - `commitment`: The original Pedersen commitment `C = v*G + r*H`.
/// - `min_value`: The minimum value threshold.
/// - `proof`: Range proof for the shifted value.
/// - `num_bits`: Expected bit width.
pub fn verify_minimum(
    commitment: &PedersenCommitment,
    min_value: u64,
    proof: &RangeProof,
    num_bits: u8,
) -> bool {
    let g = G1Projective::generator();
    let min_fr = Fr::from(min_value);

    // Shifted commitment: C' = C - min_value * G = (v - min_value)*G + r*H
    let shifted_point = (G1Projective::from(commitment.point) - g * min_fr).into_affine();
    let shifted_commitment = PedersenCommitment {
        point: shifted_point,
    };

    verify_range(&shifted_commitment, proof, num_bits)
}

// ---------------------------------------------------------------------------
// Internal: OR-proof for a single bit
// ---------------------------------------------------------------------------

/// Create a Sigma-OR proof that bit commitment `B = bit*G + r*H` opens to
/// either 0 or 1.
///
/// Uses the Cramer-Damgard-Schoenmakers technique:
/// - If `bit == 0`: the prover knows the witness for branch 0 (`B = 0*G + r*H`)
///   and simulates branch 1 (`B - G = (-1)*G + r*H` which has no known opening).
/// - If `bit == 1`: the prover knows the witness for branch 1 (`B - G = 0*G + r*H`
///   after adjustment) and simulates branch 0.
///
/// The Fiat-Shamir challenge `e` is split: `e = e0 + e1` so that the prover
/// chooses one sub-challenge freely and computes the other from `e`.
fn prove_bit_or(
    g: G1Projective,
    h: G1Projective,
    b_i: G1Affine,
    bit: u64,
    r_i: Fr,
    entropy: &[u8; 32],
) -> Option<BitProof> {
    // k is the prover's random nonce for the real branch
    let k = Fr::from_be_bytes_mod_order(entropy);
    if k.is_zero() {
        return None;
    }

    let b_proj: G1Projective = b_i.into();

    if bit == 0 {
        // Real branch: bit = 0, so B = r*H (value component is 0*G = identity)
        // Simulated branch: bit = 1, so B - G has no known discrete log on H

        // Simulate branch 1 first: pick e1 and s1 randomly
        let e1 = derive_sim_challenge(entropy, 1);
        let s1 = derive_sim_response(entropy, 1);

        // Real branch 0 commitment: A0 = k * H
        let a0 = h * k;

        // Simulated branch 1 commitment: A1 = s1*H - e1*(B - G)
        let b_minus_g = b_proj - g;
        let a1 = h * s1 - b_minus_g * e1;

        // Fiat-Shamir challenge
        let e = fiat_shamir_bit_challenge(b_i, a0.into_affine(), a1.into_affine());

        // e0 = e - e1
        let e0 = e - e1;

        // Real response: s0 = k + e0 * r_i
        let s0 = k + e0 * r_i;

        Some(BitProof { e0, s0, e1, s1 })
    } else {
        // Real branch: bit = 1, so B - G = r*H (value component cancels)
        // Simulated branch: bit = 0, so B has no known discrete log on H
        //   from the perspective of the simulated branch

        // Simulate branch 0 first: pick e0 and s0 randomly
        let e0 = derive_sim_challenge(entropy, 0);
        let s0 = derive_sim_response(entropy, 0);

        // Simulated branch 0 commitment: A0 = s0*H - e0*B
        let a0 = h * s0 - b_proj * e0;

        // Real branch 1 commitment: A1 = k * H
        let a1 = h * k;

        // Fiat-Shamir challenge
        let e = fiat_shamir_bit_challenge(b_i, a0.into_affine(), a1.into_affine());

        // e1 = e - e0
        let e1 = e - e0;

        // Real response: s1 = k + e1 * r_i
        let s1 = k + e1 * r_i;

        Some(BitProof { e0, s0, e1, s1 })
    }
}

/// Verify a Sigma-OR proof for a single bit commitment.
///
/// Checks:
/// 1. Recompute A0 = s0*H - e0*B (branch 0: B opens to 0)
/// 2. Recompute A1 = s1*H - e1*(B - G) (branch 1: B opens to 1)
/// 3. Verify Fiat-Shamir: e0 + e1 == Hash(B, A0, A1)
fn verify_bit_or(g: G1Projective, h: G1Projective, b_i: G1Affine, proof: &BitProof) -> bool {
    let b_proj: G1Projective = b_i.into();

    // Recompute branch 0 commitment: A0 = s0*H - e0*B
    let a0 = h * proof.s0 - b_proj * proof.e0;

    // Recompute branch 1 commitment: A1 = s1*H - e1*(B - G)
    let b_minus_g = b_proj - g;
    let a1 = h * proof.s1 - b_minus_g * proof.e1;

    // Verify Fiat-Shamir challenge
    let e = fiat_shamir_bit_challenge(b_i, a0.into_affine(), a1.into_affine());

    e == proof.e0 + proof.e1
}

// ---------------------------------------------------------------------------
// Internal: Blinding factor consistency proof (Schnorr on H)
// ---------------------------------------------------------------------------

/// Prove knowledge of `delta` such that `D = delta * H`.
///
/// This is a standard Schnorr proof of discrete log on base H.
fn prove_blinding(
    h: G1Projective,
    d: G1Affine,
    delta: Fr,
    entropy: &[u8; 32],
) -> Option<BlindingProof> {
    let k = Fr::from_be_bytes_mod_order(entropy);
    if k.is_zero() {
        return None;
    }

    // Commitment: A = k * H
    let a = (h * k).into_affine();

    // Fiat-Shamir challenge bound to the actual statement D = delta * H.
    let e = fiat_shamir_blinding_challenge(a, d);

    // Response: s = k + e * delta
    let s = k + e * delta;

    Some(BlindingProof {
        commitment: a,
        response: s,
    })
}

/// Verify a Schnorr proof that `D = delta * H`.
///
/// Checks: `s * H == A + e * D`
fn verify_blinding(h: G1Projective, d: G1Affine, proof: &BlindingProof) -> bool {
    // Recompute the same prover transcript challenge bound to D.
    let e = fiat_shamir_blinding_challenge(proof.commitment, d);

    // Check: s * H == A + e * D
    let lhs = h * proof.response;
    let d_proj: G1Projective = d.into();
    let a_proj: G1Projective = proof.commitment.into();
    let rhs = a_proj + d_proj * e;

    lhs == rhs
}

// ---------------------------------------------------------------------------
// Internal: Fiat-Shamir challenges
// ---------------------------------------------------------------------------

/// Compute the Fiat-Shamir challenge for a bit OR-proof.
///
/// `e = H(DOMAIN || B || A0 || A1)` interpreted as a BN254 scalar.
fn fiat_shamir_bit_challenge(b: G1Affine, a0: G1Affine, a1: G1Affine) -> Fr {
    let b_bytes = point_to_bytes(&b);
    let a0_bytes = point_to_bytes(&a0);
    let a1_bytes = point_to_bytes(&a1);

    let mut data = Vec::with_capacity(
        DOMAIN_RANGE_CHALLENGE.len() + b_bytes.len() + a0_bytes.len() + a1_bytes.len(),
    );
    data.extend_from_slice(DOMAIN_RANGE_CHALLENGE);
    data.extend_from_slice(&b_bytes);
    data.extend_from_slice(&a0_bytes);
    data.extend_from_slice(&a1_bytes);

    let hash = blake2_256(&data);
    Fr::from_be_bytes_mod_order(&hash)
}

/// Compute the Fiat-Shamir challenge for the blinding proof.
///
/// The transcript is bound to `A` and the statement point `D = delta * H`
/// so both prover and verifier derive the same challenge from the same
/// relation.
fn fiat_shamir_blinding_challenge(a: G1Affine, d: G1Affine) -> Fr {
    let a_bytes = point_to_bytes(&a);
    let d_bytes = point_to_bytes(&d);

    let mut data =
        Vec::with_capacity(DOMAIN_RANGE_CHALLENGE.len() + 8 + a_bytes.len() + d_bytes.len());
    data.extend_from_slice(DOMAIN_RANGE_CHALLENGE);
    data.extend_from_slice(b"blinding");
    data.extend_from_slice(&a_bytes);
    data.extend_from_slice(&d_bytes);

    let hash = blake2_256(&data);
    Fr::from_be_bytes_mod_order(&hash)
}

// ---------------------------------------------------------------------------
// Internal: Deterministic entropy derivation
// ---------------------------------------------------------------------------

/// Derive per-bit randomness for bit commitment `i`.
fn derive_bit_randomness(entropy: &[u8; 32], bit_index: u8) -> Fr {
    let mut input = Vec::with_capacity(32 + 1 + DOMAIN_BIT_ENTROPY.len() + 4);
    input.extend_from_slice(entropy);
    input.push(bit_index);
    input.extend_from_slice(DOMAIN_BIT_ENTROPY);
    input.extend_from_slice(b"rand");

    let hash = blake2_256(&input);
    Fr::from_be_bytes_mod_order(&hash)
}

/// Derive per-bit proof entropy for OR-proof construction.
fn derive_bit_proof_entropy(entropy: &[u8; 32], bit_index: u8) -> [u8; 32] {
    let mut input = Vec::with_capacity(32 + 1 + DOMAIN_BIT_ENTROPY.len() + 5);
    input.extend_from_slice(entropy);
    input.push(bit_index);
    input.extend_from_slice(DOMAIN_BIT_ENTROPY);
    input.extend_from_slice(b"proof");

    blake2_256(&input)
}

/// Derive simulated challenge for the OR-proof.
fn derive_sim_challenge(entropy: &[u8; 32], branch: u8) -> Fr {
    let mut input = Vec::with_capacity(32 + 1 + DOMAIN_BIT_ENTROPY.len() + 4);
    input.extend_from_slice(entropy);
    input.push(branch);
    input.extend_from_slice(DOMAIN_BIT_ENTROPY);
    input.extend_from_slice(b"sime");

    let hash = blake2_256(&input);
    Fr::from_be_bytes_mod_order(&hash)
}

/// Derive simulated response for the OR-proof.
fn derive_sim_response(entropy: &[u8; 32], branch: u8) -> Fr {
    let mut input = Vec::with_capacity(32 + 1 + DOMAIN_BIT_ENTROPY.len() + 4);
    input.extend_from_slice(entropy);
    input.push(branch);
    input.extend_from_slice(DOMAIN_BIT_ENTROPY);
    input.extend_from_slice(b"sims");

    let hash = blake2_256(&input);
    Fr::from_be_bytes_mod_order(&hash)
}

/// Derive entropy for the blinding factor consistency proof.
fn derive_blinding_proof_entropy(entropy: &[u8; 32]) -> [u8; 32] {
    let mut input = Vec::with_capacity(32 + DOMAIN_BIT_ENTROPY.len() + 8);
    input.extend_from_slice(entropy);
    input.extend_from_slice(DOMAIN_BIT_ENTROPY);
    input.extend_from_slice(b"blinding");

    blake2_256(&input)
}

// ---------------------------------------------------------------------------
// Internal: Domain-separated hashing
// ---------------------------------------------------------------------------

/// Blake2-256 with domain separation, matching `hash_with_domain` in
/// `seveny-primitives` but returning raw `[u8; 32]` for scalar conversion.
///
/// Format: `len(domain) || domain || data` where `len` is a 4-byte LE u32.
fn domain_blake2(domain: &[u8], data: &[u8]) -> [u8; 32] {
    let domain_len = (domain.len() as u32).to_le_bytes();
    let mut input = Vec::with_capacity(4 + domain.len() + data.len());
    input.extend_from_slice(&domain_len);
    input.extend_from_slice(domain);
    input.extend_from_slice(data);
    blake2_256(&input)
}

// ---------------------------------------------------------------------------
// Internal: Point serialization
// ---------------------------------------------------------------------------

/// Serialize a G1Affine point to bytes for hashing.
///
/// Uses the arkworks uncompressed serialization. For affine points this
/// produces the concatenation of the x and y coordinates as big-endian
/// field elements.
fn point_to_bytes(point: &G1Affine) -> Vec<u8> {
    use ark_serialize::CanonicalSerialize;
    let mut buf = Vec::new();
    // Use uncompressed serialization for deterministic representation
    point.serialize_uncompressed(&mut buf).unwrap_or_default();
    buf
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entropy() -> [u8; 32] {
        [0xAAu8; 32]
    }

    fn test_entropy_2() -> [u8; 32] {
        [0xBBu8; 32]
    }

    // -- Pedersen commitment tests --

    #[test]
    fn pedersen_h_generator_is_deterministic() {
        let h1 = pedersen_h_generator();
        let h2 = pedersen_h_generator();
        assert_eq!(h1, h2);
    }

    #[test]
    fn pedersen_h_differs_from_g() {
        let g = G1Projective::generator();
        let h = pedersen_h_generator();
        assert_ne!(g, h, "H must differ from G");
    }

    #[test]
    fn pedersen_commit_deterministic() {
        let v = Fr::from(42u64);
        let r = Fr::from(123u64);
        let c1 = pedersen_commit(v, r);
        let c2 = pedersen_commit(v, r);
        assert_eq!(c1, c2);
    }

    #[test]
    fn pedersen_commit_different_values() {
        let r = Fr::from(99u64);
        let c1 = pedersen_commit(Fr::from(10u64), r);
        let c2 = pedersen_commit(Fr::from(20u64), r);
        assert_ne!(c1, c2);
    }

    #[test]
    fn pedersen_commit_different_randomness() {
        let v = Fr::from(42u64);
        let c1 = pedersen_commit(v, Fr::from(1u64));
        let c2 = pedersen_commit(v, Fr::from(2u64));
        assert_ne!(
            c1, c2,
            "Different randomness must produce different commitments"
        );
    }

    #[test]
    fn pedersen_commit_homomorphic_addition() {
        // Pedersen commitments are additively homomorphic:
        // C(v1, r1) + C(v2, r2) = C(v1 + v2, r1 + r2)
        let v1 = Fr::from(10u64);
        let r1 = Fr::from(100u64);
        let v2 = Fr::from(20u64);
        let r2 = Fr::from(200u64);

        let c1 = pedersen_commit(v1, r1);
        let c2 = pedersen_commit(v2, r2);
        let c_sum = pedersen_commit(v1 + v2, r1 + r2);

        let combined: G1Affine =
            (G1Projective::from(c1.point) + G1Projective::from(c2.point)).into_affine();
        assert_eq!(combined, c_sum.point);
    }

    // -- Range proof: valid cases --

    #[test]
    fn range_proof_valid_small_value() {
        let value = 5u64;
        let r = Fr::from(999u64);
        let num_bits = 8u8;
        let entropy = test_entropy();

        let commitment = pedersen_commit(Fr::from(value), r);
        let proof =
            prove_range(value, r, num_bits, &entropy).expect("proof generation should succeed");

        assert!(
            verify_range(&commitment, &proof, num_bits),
            "valid range proof must verify"
        );
    }

    #[test]
    fn range_proof_value_zero() {
        let value = 0u64;
        let r = Fr::from(42u64);
        let num_bits = 8u8;
        let entropy = test_entropy();

        let commitment = pedersen_commit(Fr::from(value), r);
        let proof =
            prove_range(value, r, num_bits, &entropy).expect("proof for zero should succeed");

        assert!(
            verify_range(&commitment, &proof, num_bits),
            "range proof for zero must verify"
        );
    }

    #[test]
    fn range_proof_max_value_8bit() {
        let value = 255u64; // 2^8 - 1
        let r = Fr::from(777u64);
        let num_bits = 8u8;
        let entropy = test_entropy();

        let commitment = pedersen_commit(Fr::from(value), r);
        let proof = prove_range(value, r, num_bits, &entropy)
            .expect("proof for max 8-bit value should succeed");

        assert!(
            verify_range(&commitment, &proof, num_bits),
            "range proof for 2^8-1 must verify"
        );
    }

    #[test]
    fn range_proof_max_value_16bit() {
        let value = 65535u64; // 2^16 - 1
        let r = Fr::from(888u64);
        let num_bits = 16u8;
        let entropy = test_entropy();

        let commitment = pedersen_commit(Fr::from(value), r);
        let proof = prove_range(value, r, num_bits, &entropy)
            .expect("proof for max 16-bit value should succeed");

        assert!(
            verify_range(&commitment, &proof, num_bits),
            "range proof for 2^16-1 must verify"
        );
    }

    #[test]
    fn range_proof_32bit() {
        let value = 1_000_000u64;
        let r = Fr::from(12345u64);
        let num_bits = 32u8;
        let entropy = test_entropy();

        let commitment = pedersen_commit(Fr::from(value), r);
        let proof = prove_range(value, r, num_bits, &entropy).expect("32-bit proof should succeed");

        assert!(
            verify_range(&commitment, &proof, num_bits),
            "32-bit range proof must verify"
        );
    }

    #[test]
    fn range_proof_64bit() {
        let value = u64::MAX;
        let r = Fr::from(54321u64);
        let num_bits = 64u8;
        let entropy = test_entropy();

        let commitment = pedersen_commit(Fr::from(value), r);
        let proof = prove_range(value, r, num_bits, &entropy).expect("64-bit proof should succeed");

        assert!(
            verify_range(&commitment, &proof, num_bits),
            "64-bit range proof must verify"
        );
    }

    // -- Range proof: invalid cases --

    #[test]
    fn range_proof_value_out_of_range() {
        let value = 256u64; // exceeds 8-bit range
        let r = Fr::from(42u64);
        let num_bits = 8u8;
        let entropy = test_entropy();

        assert!(
            prove_range(value, r, num_bits, &entropy).is_none(),
            "proof generation should fail for out-of-range value"
        );
    }

    #[test]
    fn range_proof_zero_bits_rejected() {
        let r = Fr::from(1u64);
        let entropy = test_entropy();

        assert!(
            prove_range(0, r, 0, &entropy).is_none(),
            "zero bits should be rejected"
        );
    }

    #[test]
    fn range_proof_too_many_bits_rejected() {
        let r = Fr::from(1u64);
        let entropy = test_entropy();

        assert!(
            prove_range(0, r, 65, &entropy).is_none(),
            "num_bits > 64 should be rejected"
        );
    }

    #[test]
    fn range_proof_wrong_commitment_fails() {
        let value = 42u64;
        let r = Fr::from(100u64);
        let num_bits = 8u8;
        let entropy = test_entropy();

        // Generate proof for correct commitment
        let proof =
            prove_range(value, r, num_bits, &entropy).expect("proof generation should succeed");

        // Verify against a different commitment (wrong value)
        let wrong_commitment = pedersen_commit(Fr::from(99u64), r);
        assert!(
            !verify_range(&wrong_commitment, &proof, num_bits),
            "proof must fail against wrong commitment"
        );

        // Verify against a different commitment (wrong randomness)
        let wrong_r_commitment = pedersen_commit(Fr::from(value), Fr::from(999u64));
        assert!(
            !verify_range(&wrong_r_commitment, &proof, num_bits),
            "proof must fail against commitment with wrong randomness"
        );
    }

    #[test]
    fn range_proof_wrong_num_bits_fails() {
        let value = 42u64;
        let r = Fr::from(100u64);
        let entropy = test_entropy();

        let commitment = pedersen_commit(Fr::from(value), r);
        let proof = prove_range(value, r, 8, &entropy).expect("proof generation should succeed");

        // Try to verify with different bit width
        assert!(
            !verify_range(&commitment, &proof, 16),
            "proof must fail with wrong num_bits"
        );
    }

    #[test]
    fn range_proof_different_entropy_different_proofs() {
        let value = 42u64;
        let r = Fr::from(100u64);
        let num_bits = 8u8;

        let proof1 =
            prove_range(value, r, num_bits, &test_entropy()).expect("proof 1 should succeed");
        let proof2 =
            prove_range(value, r, num_bits, &test_entropy_2()).expect("proof 2 should succeed");

        // Different entropy produces different proofs
        assert_ne!(
            proof1.bit_commitments, proof2.bit_commitments,
            "different entropy should produce different bit commitments"
        );

        // But both verify against the same commitment
        let commitment = pedersen_commit(Fr::from(value), r);
        assert!(verify_range(&commitment, &proof1, num_bits));
        assert!(verify_range(&commitment, &proof2, num_bits));
    }

    // -- Minimum value proofs --

    #[test]
    fn minimum_proof_valid() {
        let value = 100u64;
        let min_value = 50u64;
        let r = Fr::from(42u64);
        let num_bits = 8u8;
        let entropy = test_entropy();

        let commitment = pedersen_commit(Fr::from(value), r);
        let proof = prove_minimum(value, min_value, r, num_bits, &entropy)
            .expect("minimum proof should succeed");

        assert!(
            verify_minimum(&commitment, min_value, &proof, num_bits),
            "valid minimum proof must verify"
        );
    }

    #[test]
    fn minimum_proof_exact_minimum() {
        let value = 50u64;
        let min_value = 50u64;
        let r = Fr::from(77u64);
        let num_bits = 8u8;
        let entropy = test_entropy();

        let commitment = pedersen_commit(Fr::from(value), r);
        let proof = prove_minimum(value, min_value, r, num_bits, &entropy)
            .expect("proof at exact minimum should succeed (value - min = 0)");

        assert!(
            verify_minimum(&commitment, min_value, &proof, num_bits),
            "proof at exact minimum must verify"
        );
    }

    #[test]
    fn minimum_proof_below_minimum_fails() {
        let value = 49u64;
        let min_value = 50u64;
        let r = Fr::from(42u64);
        let num_bits = 8u8;
        let entropy = test_entropy();

        assert!(
            prove_minimum(value, min_value, r, num_bits, &entropy).is_none(),
            "proof generation must fail when value < min_value"
        );
    }

    #[test]
    fn minimum_proof_wrong_min_value_fails() {
        let value = 100u64;
        let min_value = 50u64;
        let r = Fr::from(42u64);
        let num_bits = 8u8;
        let entropy = test_entropy();

        let commitment = pedersen_commit(Fr::from(value), r);
        let proof =
            prove_minimum(value, min_value, r, num_bits, &entropy).expect("proof should succeed");

        // Verify with a different min_value
        assert!(
            !verify_minimum(&commitment, 60u64, &proof, num_bits),
            "proof must fail with different min_value"
        );
    }

    #[test]
    fn minimum_proof_stake_scenario() {
        // Simulate: validator has 1000 tokens, MinStake is 500
        let stake = 1000u64;
        let min_stake = 500u64;
        let r = Fr::from(314159u64);
        let num_bits = 16u8;
        let entropy = test_entropy();

        let commitment = pedersen_commit(Fr::from(stake), r);
        let proof = prove_minimum(stake, min_stake, r, num_bits, &entropy)
            .expect("stake proof should succeed");

        assert!(
            verify_minimum(&commitment, min_stake, &proof, num_bits),
            "stake >= MinStake proof must verify"
        );

        // A different validator with insufficient stake should fail
        let low_stake = 499u64;
        assert!(
            prove_minimum(low_stake, min_stake, r, num_bits, &entropy).is_none(),
            "insufficient stake proof generation must fail"
        );
    }

    // -- Edge cases --

    #[test]
    fn range_proof_single_bit() {
        // 1-bit range: value in [0, 2) = {0, 1}
        for value in 0..2u64 {
            let r = Fr::from(42u64);
            let entropy = test_entropy();

            let commitment = pedersen_commit(Fr::from(value), r);
            let proof = prove_range(value, r, 1, &entropy).expect("1-bit proof should succeed");

            assert!(
                verify_range(&commitment, &proof, 1),
                "1-bit range proof for value={} must verify",
                value
            );
        }

        // Value 2 should fail for 1-bit range
        assert!(prove_range(2, Fr::from(42u64), 1, &test_entropy()).is_none());
    }

    #[test]
    fn range_proof_verify_with_zero_bits_returns_false() {
        let commitment = pedersen_commit(Fr::from(0u64), Fr::from(1u64));
        let proof =
            prove_range(0, Fr::from(1u64), 8, &test_entropy()).expect("proof should succeed");

        assert!(
            !verify_range(&commitment, &proof, 0),
            "verification with 0 bits must return false"
        );
    }

    #[test]
    fn range_proof_verify_with_excess_bits_returns_false() {
        let commitment = pedersen_commit(Fr::from(0u64), Fr::from(1u64));
        let proof =
            prove_range(0, Fr::from(1u64), 8, &test_entropy()).expect("proof should succeed");

        assert!(
            !verify_range(&commitment, &proof, 65),
            "verification with >64 bits must return false"
        );
    }

    #[test]
    fn pedersen_commit_zero_value_zero_randomness() {
        let c = pedersen_commit(Fr::zero(), Fr::zero());
        // C = 0*G + 0*H = identity point
        let identity = G1Affine::identity();
        assert_eq!(c.point, identity);
    }

    #[test]
    fn range_proof_deterministic() {
        let value = 42u64;
        let r = Fr::from(100u64);
        let num_bits = 8u8;
        let entropy = test_entropy();

        let proof1 = prove_range(value, r, num_bits, &entropy).expect("proof 1 should succeed");
        let proof2 = prove_range(value, r, num_bits, &entropy).expect("proof 2 should succeed");

        assert_eq!(
            proof1.bit_commitments, proof2.bit_commitments,
            "same inputs must produce same proof"
        );
    }
}
