//! Cryptographic primitives for presence verification and state proofs.

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{blake2_256, H256};
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

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
#[derive(Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug)]
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, RuntimeDebug)]
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
    Clone, Copy, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug, Hash,
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
    Clone, Copy, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug, Default,
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, RuntimeDebug)]
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, RuntimeDebug)]
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
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, RuntimeDebug)]
pub struct ShareIndex(pub u8);

#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, RuntimeDebug)]
pub struct Share {
    pub index: ShareIndex,
    pub value: [u8; 32],
}

/// Polynomial evaluation for Shamir's scheme using big-integer arithmetic.
#[allow(clippy::cast_possible_truncation)]
pub fn eval_polynomial(coeffs: &[[u8; 32]], x: u8) -> [u8; 32] {
    let mut result = [0u8; 32];

    for coeff in coeffs.iter().rev() {
        let mut carry: u16 = 0;
        for byte in &mut result {
            let prod = u16::from(*byte) * u16::from(x) + carry;
            *byte = prod as u8;
            carry = prod >> 8;
        }

        let mut carry: u16 = 0;
        for (i, byte) in result.iter_mut().enumerate() {
            let sum = u16::from(*byte) + u16::from(coeff[i]) + carry;
            *byte = sum as u8;
            carry = sum >> 8;
        }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
