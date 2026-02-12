//! Cryptographic primitives for presence verification and state proofs.

use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{blake2_256, H256};
use sp_runtime::RuntimeDebug;

use crate::traits::{ConstantTimeEq, CryptoHash, DomainSeparatedHash};

// Domain separators for hash functions
pub const DOMAIN_PRESENCE: &[u8] = b"7ay:presence:v1";
pub const DOMAIN_EPOCH: &[u8] = b"7ay:epoch:v1";
pub const DOMAIN_COMMITMENT: &[u8] = b"7ay:commit:v1";
pub const DOMAIN_MERKLE: &[u8] = b"7ay:merkle:v1";
pub const DOMAIN_NULLIFIER: &[u8] = b"7ay:nullifier:v1";

/// Hash with domain separation.
#[inline]
pub fn hash_with_domain(domain: &[u8], data: &[u8]) -> H256 {
    let mut input = Vec::with_capacity(domain.len() + data.len());
    input.extend_from_slice(domain);
    input.extend_from_slice(data);
    H256(blake2_256(&input))
}

/// Hash two values together (for Merkle trees).
#[inline]
pub fn hash_pair(left: &H256, right: &H256) -> H256 {
    let mut input = [0u8; 64];
    input[..32].copy_from_slice(left.as_bytes());
    input[32..].copy_from_slice(right.as_bytes());
    H256(blake2_256(&input))
}

/// Pedersen-style commitment: C = H(domain || value || randomness)
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub struct Commitment(pub H256);

impl Commitment {
    pub fn new<V: Encode>(value: &V, randomness: &[u8; 32]) -> Self {
        let value_bytes = value.encode();
        let mut input = Vec::with_capacity(DOMAIN_COMMITMENT.len() + value_bytes.len() + 32);
        input.extend_from_slice(DOMAIN_COMMITMENT);
        input.extend_from_slice(&value_bytes);
        input.extend_from_slice(randomness);
        Self(H256(blake2_256(&input)))
    }

    pub fn verify<V: Encode>(&self, value: &V, randomness: &[u8; 32]) -> bool {
        let expected = Self::new(value, randomness);
        self.0.ct_eq(&expected.0)
    }

    pub const fn as_h256(&self) -> &H256 {
        &self.0
    }
}

/// Merkle proof for membership verification.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct MerkleProof {
    pub leaf_index: u64,
    pub siblings: Vec<H256>,
}

impl MerkleProof {
    /// Verify membership against a root.
    pub fn verify(&self, root: &H256, leaf: &H256) -> bool {
        let mut current = *leaf;
        let mut index = self.leaf_index;

        for sibling in &self.siblings {
            current = if index & 1 == 0 {
                hash_pair(&current, sibling)
            } else {
                hash_pair(sibling, &current)
            };
            index >>= 1;
        }

        current.ct_eq(root)
    }

    /// Compute the root from a leaf and proof.
    pub fn compute_root(&self, leaf: &H256) -> H256 {
        let mut current = *leaf;
        let mut index = self.leaf_index;

        for sibling in &self.siblings {
            current = if index & 1 == 0 {
                hash_pair(&current, sibling)
            } else {
                hash_pair(sibling, &current)
            };
            index >>= 1;
        }

        current
    }
}

/// Nullifier to prevent double-spending/double-presence.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
    Hash,
)]
pub struct Nullifier(pub H256);

impl Nullifier {
    pub fn derive(secret: &[u8; 32], epoch_id: u64, nonce: u64) -> Self {
        let mut input = Vec::with_capacity(DOMAIN_NULLIFIER.len() + 32 + 16);
        input.extend_from_slice(DOMAIN_NULLIFIER);
        input.extend_from_slice(secret);
        input.extend_from_slice(&epoch_id.to_le_bytes());
        input.extend_from_slice(&nonce.to_le_bytes());
        Self(H256(blake2_256(&input)))
    }
}

/// State root representing a snapshot of all presence data.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
    Default,
)]
pub struct StateRoot(pub H256);

impl StateRoot {
    pub const EMPTY: Self = Self(H256([0u8; 32]));

    pub fn from_leaves(leaves: &[H256]) -> Self {
        if leaves.is_empty() {
            return Self::EMPTY;
        }

        let mut layer: Vec<H256> = leaves.to_vec();

        // Pad to power of 2
        let next_pow2 = layer.len().next_power_of_two();
        while layer.len() < next_pow2 {
            layer.push(H256::zero());
        }

        // Build tree bottom-up
        while layer.len() > 1 {
            let mut next_layer = Vec::with_capacity(layer.len() / 2);
            for chunk in layer.chunks(2) {
                next_layer.push(hash_pair(&chunk[0], &chunk[1]));
            }
            layer = next_layer;
        }

        Self(layer[0])
    }
}

/// Presence proof combining commitment and Merkle proof.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct PresenceProof {
    pub commitment: Commitment,
    pub merkle_proof: MerkleProof,
    pub nullifier: Nullifier,
}

impl PresenceProof {
    pub fn verify(&self, state_root: &StateRoot, commitment_leaf: &H256) -> bool {
        // Verify the commitment is included in the state
        self.merkle_proof.verify(&state_root.0, commitment_leaf)
    }
}

/// ZK statement for presence verification.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct PresenceStatement {
    pub epoch_id: u64,
    pub state_root: StateRoot,
    pub nullifier: Nullifier,
}

/// ZK witness (private inputs) for presence proof generation.
pub struct PresenceWitness {
    pub secret: [u8; 32],
    pub randomness: [u8; 32],
    pub merkle_path: Vec<H256>,
    pub leaf_index: u64,
}

/// Shamir secret sharing types for key distribution.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct ShareIndex(pub u8);

#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct Share {
    pub index: ShareIndex,
    pub value: [u8; 32],
}

pub mod gf256 {
    const RIJNDAEL_POLY: u16 = 0x11B;

    #[inline]
    pub fn mul(a: u8, b: u8) -> u8 {
        let mut result: u8 = 0;
        let mut a = a;
        let mut b = b;

        while b != 0 {
            if b & 1 != 0 {
                result ^= a;
            }
            let high_bit = a & 0x80;
            a <<= 1;
            if high_bit != 0 {
                a ^= (RIJNDAEL_POLY & 0xFF) as u8;
            }
            b >>= 1;
        }

        result
    }

    #[inline]
    pub fn add(a: u8, b: u8) -> u8 {
        a ^ b
    }

    #[inline]
    pub fn sub(a: u8, b: u8) -> u8 {
        a ^ b
    }

    pub fn inv(a: u8) -> u8 {
        if a == 0 {
            return 0;
        }

        let mut result = a;
        for _ in 0..6 {
            result = mul(result, result);
            result = mul(result, a);
        }
        mul(result, result)
    }

    #[inline]
    pub fn div(a: u8, b: u8) -> u8 {
        if b == 0 {
            return 0;
        }
        mul(a, inv(b))
    }

    pub fn pow(base: u8, exp: u8) -> u8 {
        if exp == 0 {
            return 1;
        }
        let mut result = 1u8;
        let mut base = base;
        let mut exp = exp;
        while exp > 0 {
            if exp & 1 != 0 {
                result = mul(result, base);
            }
            base = mul(base, base);
            exp >>= 1;
        }
        result
    }
}

pub fn eval_polynomial(coeffs: &[[u8; 32]], x: u8) -> [u8; 32] {
    let mut result = [0u8; 32];

    for byte_idx in 0..32 {
        let mut value = 0u8;
        for coeff in coeffs.iter().rev() {
            value = gf256::add(gf256::mul(value, x), coeff[byte_idx]);
        }
        result[byte_idx] = value;
    }

    result
}

impl CryptoHash for Commitment {
    fn crypto_hash(&self) -> H256 {
        self.0
    }
}

impl DomainSeparatedHash for Commitment {
    const DOMAIN: &'static [u8] = DOMAIN_COMMITMENT;

    fn domain_hash(&self) -> H256 {
        hash_with_domain(Self::DOMAIN, self.0.as_bytes())
    }
}

pub const DOMAIN_SHARE: &[u8] = b"7ay:share:v1";
pub const DOMAIN_VSS: &[u8] = b"7ay:vss:v1";

pub struct ShamirScheme;

impl ShamirScheme {
    pub fn split(secret: &[u8; 32], threshold: u8, total: u8) -> Option<Vec<Share>> {
        if threshold < 2 || total < threshold || total == 0 {
            return None;
        }

        let mut shares = Vec::with_capacity(total as usize);
        let mut coefficients = Vec::with_capacity(threshold as usize);
        coefficients.push(*secret);

        for i in 1..threshold {
            let mut coeff = [0u8; 32];
            let seed_input = [&secret[..], &[i][..]].concat();
            let hash = blake2_256(&seed_input);
            coeff.copy_from_slice(&hash);
            coefficients.push(coeff);
        }

        for idx in 1..=total {
            let share_value = eval_polynomial(&coefficients, idx);
            shares.push(Share {
                index: ShareIndex(idx),
                value: share_value,
            });
        }

        Some(shares)
    }

    pub fn reconstruct(shares: &[Share], threshold: u8) -> Option<[u8; 32]> {
        if shares.len() < threshold as usize {
            return None;
        }

        Some(Self::reconstruct_inner(&shares[..threshold as usize]))
    }

    fn reconstruct_inner(shares: &[Share]) -> [u8; 32] {
        let mut secret = [0u8; 32];

        for byte_idx in 0..32 {
            let mut result: u8 = 0;

            for (i, share_i) in shares.iter().enumerate() {
                let xi = share_i.index.0;
                let yi = share_i.value[byte_idx];
                let li = Self::compute_lagrange_basis_at_zero(shares, i, xi);
                result = gf256::add(result, gf256::mul(yi, li));
            }

            secret[byte_idx] = result;
        }

        secret
    }

    fn compute_lagrange_basis_at_zero(shares: &[Share], i: usize, xi: u8) -> u8 {
        let mut result: u8 = 1;

        for (j, share_j) in shares.iter().enumerate() {
            if i != j {
                let xj = share_j.index.0;
                let denominator = gf256::sub(xi, xj);

                if denominator == 0 {
                    continue;
                }

                result = gf256::mul(result, gf256::div(xj, denominator));
            }
        }

        result
    }

    pub fn verify_share(share: &Share, commitment: &H256) -> bool {
        let share_hash = Self::hash_share(share);
        share_hash.ct_eq(commitment)
    }

    pub fn hash_share(share: &Share) -> H256 {
        let mut input = Vec::with_capacity(DOMAIN_SHARE.len() + 33);
        input.extend_from_slice(DOMAIN_SHARE);
        input.push(share.index.0);
        input.extend_from_slice(&share.value);
        H256(blake2_256(&input))
    }

    pub fn create_commitment(share: &Share) -> H256 {
        Self::hash_share(share)
    }
}

#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
)]
pub struct VssCommitment {
    pub coefficients: Vec<H256>,
}

pub struct FeldmanVSS;

impl FeldmanVSS {
    pub fn share_with_commitments(
        secret: &[u8; 32],
        threshold: u8,
        total: u8,
    ) -> Option<(Vec<Share>, VssCommitment)> {
        if threshold < 2 || total < threshold {
            return None;
        }

        let mut coefficients_hash = Vec::with_capacity(threshold as usize);

        let secret_commitment = hash_with_domain(DOMAIN_VSS, secret);
        coefficients_hash.push(secret_commitment);

        for i in 1..threshold {
            let seed_input = [&secret[..], &[i][..]].concat();
            let coeff_commitment = hash_with_domain(DOMAIN_VSS, &seed_input);
            coefficients_hash.push(coeff_commitment);
        }

        let shares = ShamirScheme::split(secret, threshold, total)?;

        Some((
            shares,
            VssCommitment {
                coefficients: coefficients_hash,
            },
        ))
    }

    pub fn verify_share_against_commitments(
        share: &Share,
        commitments: &VssCommitment,
        threshold: u8,
    ) -> bool {
        if commitments.coefficients.len() != threshold as usize {
            return false;
        }
        if share.index.0 == 0 {
            return false;
        }

        let share_commitment = ShamirScheme::create_commitment(share);

        let mut expected_input = Vec::with_capacity(commitments.coefficients.len() * 32 + 1);
        expected_input.push(share.index.0);
        for coeff in &commitments.coefficients {
            expected_input.extend_from_slice(coeff.as_bytes());
        }

        let expected_hash = hash_with_domain(DOMAIN_VSS, &expected_input);
        let share_hash = hash_with_domain(DOMAIN_VSS, share_commitment.as_bytes());

        expected_hash.ct_eq(&share_hash)
    }

    pub fn verify_share_count(shares: &[Share], threshold: u8) -> bool {
        shares.len() >= threshold as usize
    }
}

impl Share {
    pub fn new(index: u8, value: [u8; 32]) -> Self {
        Self {
            index: ShareIndex(index),
            value,
        }
    }
}

impl ShareIndex {
    pub fn value(&self) -> u8 {
        self.0
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn gf256_add_is_xor() {
        assert_eq!(gf256::add(0, 0), 0);
        assert_eq!(gf256::add(0xFF, 0xFF), 0);
        assert_eq!(gf256::add(0xAB, 0xCD), 0xAB ^ 0xCD);
    }

    #[test]
    fn gf256_mul_basic() {
        assert_eq!(gf256::mul(0, 5), 0);
        assert_eq!(gf256::mul(1, 5), 5);
        assert_eq!(gf256::mul(5, 1), 5);
        assert_eq!(gf256::mul(2, 2), 4);
    }

    #[test]
    fn gf256_mul_overflow() {
        let result = gf256::mul(0x80, 2);
        assert_eq!(result, 0x1B);
    }

    #[test]
    fn gf256_inverse() {
        for a in 1..=255u8 {
            let inv = gf256::inv(a);
            assert_eq!(gf256::mul(a, inv), 1);
        }
    }

    #[test]
    fn gf256_div() {
        for a in 1..=255u8 {
            assert_eq!(gf256::div(a, a), 1);
        }
        for a in 0..=255u8 {
            assert_eq!(gf256::div(a, 1), a);
        }
    }

    #[test]
    fn gf256_pow() {
        assert_eq!(gf256::pow(2, 0), 1);
        assert_eq!(gf256::pow(2, 1), 2);
        assert_eq!(gf256::pow(2, 8), gf256::mul(gf256::pow(2, 7), 2));
    }

    #[test]
    fn shamir_roundtrip_simple() {
        let secret = [42u8; 32];
        let shares = ShamirScheme::split(&secret, 2, 3).expect("split failed");
        let reconstructed = ShamirScheme::reconstruct(&shares[0..2], 2).expect("reconstruct failed");
        assert_eq!(secret, reconstructed);
    }

    #[test]
    fn shamir_roundtrip_random_secret() {
        let secret: [u8; 32] = [
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
            0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        ];
        let shares = ShamirScheme::split(&secret, 3, 5).expect("split failed");

        let combos: [(usize, usize, usize); 4] = [
            (0, 1, 2),
            (0, 2, 4),
            (1, 3, 4),
            (2, 3, 4),
        ];

        for (a, b, c) in combos {
            let subset = vec![shares[a].clone(), shares[b].clone(), shares[c].clone()];
            let reconstructed = ShamirScheme::reconstruct(&subset, 3).expect("reconstruct failed");
            assert_eq!(secret, reconstructed);
        }
    }

    #[test]
    fn shamir_threshold_boundary() {
        let secret = [0xAB; 32];
        let shares = ShamirScheme::split(&secret, 4, 7).expect("split failed");

        let reconstructed = ShamirScheme::reconstruct(&shares[0..4], 4).expect("reconstruct failed");
        assert_eq!(secret, reconstructed);

        let reconstructed = ShamirScheme::reconstruct(&shares[0..6], 4).expect("reconstruct failed");
        assert_eq!(secret, reconstructed);
    }

    #[test]
    fn commitment_verify() {
        let value = 42u64;
        let randomness = [1u8; 32];

        let commitment = Commitment::new(&value, &randomness);
        assert!(commitment.verify(&value, &randomness));
        assert!(!commitment.verify(&43u64, &randomness));
    }

    #[test]
    fn merkle_proof_single_leaf() {
        let leaf = H256::repeat_byte(0x01);
        let proof = MerkleProof {
            leaf_index: 0,
            siblings: vec![],
        };

        assert!(proof.verify(&leaf, &leaf));
    }

    #[test]
    fn merkle_proof_two_leaves() {
        let left = H256::repeat_byte(0x01);
        let right = H256::repeat_byte(0x02);
        let root = hash_pair(&left, &right);

        let proof_left = MerkleProof {
            leaf_index: 0,
            siblings: vec![right],
        };
        assert!(proof_left.verify(&root, &left));

        let proof_right = MerkleProof {
            leaf_index: 1,
            siblings: vec![left],
        };
        assert!(proof_right.verify(&root, &right));
    }

    #[test]
    fn nullifier_uniqueness() {
        let secret = [42u8; 32];
        let n1 = Nullifier::derive(&secret, 1, 0);
        let n2 = Nullifier::derive(&secret, 1, 1);
        let n3 = Nullifier::derive(&secret, 2, 0);

        assert_ne!(n1, n2);
        assert_ne!(n1, n3);
        assert_ne!(n2, n3);
    }

    #[test]
    fn state_root_empty() {
        let root = StateRoot::from_leaves(&[]);
        assert_eq!(root, StateRoot::EMPTY);
    }

    #[test]
    fn state_root_deterministic() {
        let leaves = vec![
            H256::repeat_byte(0x01),
            H256::repeat_byte(0x02),
            H256::repeat_byte(0x03),
        ];

        let root1 = StateRoot::from_leaves(&leaves);
        let root2 = StateRoot::from_leaves(&leaves);

        assert_eq!(root1, root2);
    }

    #[test]
    fn shamir_split_creates_shares() {
        let secret = [42u8; 32];
        let shares = ShamirScheme::split(&secret, 2, 3).expect("split failed");
        assert_eq!(shares.len(), 3);
        assert_eq!(shares[0].index.0, 1);
        assert_eq!(shares[1].index.0, 2);
        assert_eq!(shares[2].index.0, 3);
    }

    #[test]
    fn shamir_shares_are_different() {
        let secret = [42u8; 32];
        let shares = ShamirScheme::split(&secret, 2, 3).expect("split failed");
        assert_ne!(shares[0].value, shares[1].value);
        assert_ne!(shares[1].value, shares[2].value);
    }

    #[test]
    fn shamir_reconstruct_returns_result() {
        let secret = [42u8; 32];
        let shares = ShamirScheme::split(&secret, 2, 3).expect("split failed");
        let result = ShamirScheme::reconstruct(&shares[0..2], 2);
        assert!(result.is_some());
    }

    #[test]
    fn shamir_reconstruct_deterministic() {
        let secret = [42u8; 32];
        let shares = ShamirScheme::split(&secret, 2, 3).expect("split failed");
        let result1 = ShamirScheme::reconstruct(&shares[0..2], 2);
        let result2 = ShamirScheme::reconstruct(&shares[0..2], 2);
        assert_eq!(result1, result2);
    }

    #[test]
    fn shamir_insufficient_shares() {
        let secret = [1u8; 32];
        let shares = ShamirScheme::split(&secret, 3, 5).expect("split failed");

        let result = ShamirScheme::reconstruct(&shares[0..2], 3);
        assert!(result.is_none());
    }

    #[test]
    fn shamir_invalid_parameters() {
        let secret = [1u8; 32];
        assert!(ShamirScheme::split(&secret, 1, 3).is_none());
        assert!(ShamirScheme::split(&secret, 5, 3).is_none());
        assert!(ShamirScheme::split(&secret, 2, 0).is_none());
    }

    #[test]
    fn shamir_share_commitment() {
        let secret = [5u8; 32];
        let shares = ShamirScheme::split(&secret, 2, 3).expect("split failed");

        let commitment = ShamirScheme::create_commitment(&shares[0]);
        assert!(ShamirScheme::verify_share(&shares[0], &commitment));
        assert!(!ShamirScheme::verify_share(&shares[1], &commitment));
    }

    #[test]
    fn feldman_vss_creates_shares_and_commitments() {
        let secret = [99u8; 32];
        let result = FeldmanVSS::share_with_commitments(&secret, 2, 3);
        assert!(result.is_some());

        let (shares, commitments) = result.expect("vss failed");
        assert_eq!(shares.len(), 3);
        assert_eq!(commitments.coefficients.len(), 2);
    }

    #[test]
    fn feldman_vss_invalid_parameters() {
        let secret = [1u8; 32];
        assert!(FeldmanVSS::share_with_commitments(&secret, 1, 3).is_none());
        assert!(FeldmanVSS::share_with_commitments(&secret, 5, 3).is_none());
    }

    #[test]
    fn feldman_verify_share_count() {
        let secret = [1u8; 32];
        let (shares, _) = FeldmanVSS::share_with_commitments(&secret, 3, 5).expect("vss failed");

        assert!(FeldmanVSS::verify_share_count(&shares, 3));
        assert!(FeldmanVSS::verify_share_count(&shares, 5));
        assert!(!FeldmanVSS::verify_share_count(&shares[0..2], 3));
    }

    #[test]
    fn share_index_value() {
        let index = ShareIndex(5);
        assert_eq!(index.value(), 5);
    }

    #[test]
    fn share_new() {
        let share = Share::new(3, [7u8; 32]);
        assert_eq!(share.index.0, 3);
        assert_eq!(share.value, [7u8; 32]);
    }

    #[test]
    fn different_secrets_different_shares() {
        let secret1 = [1u8; 32];
        let secret2 = [2u8; 32];

        let shares1 = ShamirScheme::split(&secret1, 2, 3).expect("split failed");
        let shares2 = ShamirScheme::split(&secret2, 2, 3).expect("split failed");

        assert_ne!(shares1[0].value, shares2[0].value);
    }

    #[test]
    fn share_indices_are_sequential() {
        let secret = [1u8; 32];
        let shares = ShamirScheme::split(&secret, 2, 5).expect("split failed");

        for (i, share) in shares.iter().enumerate() {
            assert_eq!(share.index.0, (i + 1) as u8);
        }
    }
}
