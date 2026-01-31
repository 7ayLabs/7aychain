//! Protocol trait abstractions for cryptographic operations and state management.

use parity_scale_codec::{Decode, Encode};
use sp_core::H256;
use sp_std::vec::Vec;

/// Cryptographic hash computation.
pub trait CryptoHash {
    fn crypto_hash(&self) -> H256;
}

/// Domain-separated hashing to prevent cross-protocol collisions.
pub trait DomainSeparatedHash {
    const DOMAIN: &'static [u8];
    fn domain_hash(&self) -> H256;
}

/// Cryptographic commitment with hiding and binding properties.
pub trait Commitment: Sized + Encode + Decode + Clone {
    type Value: Encode;
    type Randomness: Encode;
    type Opening: Encode + Decode;

    fn commit(value: &Self::Value, randomness: &Self::Randomness) -> Self;
    fn verify(&self, value: &Self::Value, opening: &Self::Opening) -> bool;
    fn as_bytes(&self) -> &[u8];
}

/// Merkle tree for O(log n) membership proofs.
pub trait MerkleTree {
    type Leaf: Encode;
    type Proof: Encode + Decode;

    fn root(&self) -> H256;
    fn prove(&self, index: usize) -> Option<Self::Proof>;
    fn verify_proof(root: &H256, leaf: &Self::Leaf, proof: &Self::Proof) -> bool;
}

/// Zero-knowledge proof system interface.
pub trait ZkProof {
    type Statement: Encode + Decode;
    type Witness;
    type Proof: Encode + Decode;

    fn prove(statement: &Self::Statement, witness: &Self::Witness) -> Option<Self::Proof>;
    fn verify(statement: &Self::Statement, proof: &Self::Proof) -> bool;
}

/// Threshold secret sharing (t-of-n).
pub trait SecretSharing {
    type Secret;
    type Share: Encode + Decode + Clone;
    type Index: Encode + Decode + Copy;

    fn split(
        secret: &Self::Secret,
        threshold: u32,
        total: u32,
    ) -> Option<Vec<(Self::Index, Self::Share)>>;
    fn reconstruct(shares: &[(Self::Index, Self::Share)]) -> Option<Self::Secret>;
    fn verify_share(index: &Self::Index, share: &Self::Share, commitment: &H256) -> bool;
}

/// Verifiable state transition.
pub trait StateTransition {
    type State: Encode + Decode + Clone;
    type Action: Encode + Decode;
    type Proof: Encode + Decode;

    fn apply(state: &Self::State, action: &Self::Action) -> Option<Self::State>;
    fn prove(pre: &Self::State, action: &Self::Action, post: &Self::State) -> Option<Self::Proof>;
    fn verify(
        pre_root: &H256,
        post_root: &H256,
        action: &Self::Action,
        proof: &Self::Proof,
    ) -> bool;
}

/// Digital signature scheme.
pub trait Signature {
    type PublicKey: Encode + Decode + Clone;
    type SecretKey;
    type Sig: Encode + Decode + Clone;

    fn sign(sk: &Self::SecretKey, msg: &[u8]) -> Self::Sig;
    fn verify(pk: &Self::PublicKey, msg: &[u8], sig: &Self::Sig) -> bool;
}

/// Aggregate signature for batch verification.
pub trait AggregateSignature: Signature {
    type AggregateSig: Encode + Decode;

    fn aggregate(signatures: &[Self::Sig]) -> Option<Self::AggregateSig>;
    fn verify_aggregate(pks: &[Self::PublicKey], msgs: &[&[u8]], agg: &Self::AggregateSig) -> bool;
}

/// Chain binding for replay protection.
pub trait ChainBound {
    fn bind(&self, chain_id: u64, block_hash: H256, block_num: u64) -> H256;
    fn verify_binding(
        &self,
        binding: &H256,
        chain_id: u64,
        block_hash: H256,
        block_num: u64,
    ) -> bool;
}

/// Epoch-bound data validation.
pub trait EpochBound {
    type EpochId: Copy + PartialEq;

    fn epoch(&self) -> Self::EpochId;
    fn valid_in(&self, epoch: Self::EpochId) -> bool {
        self.epoch() == epoch
    }
}

/// Invariant checking.
pub trait Invariant {
    type ViolationId;

    fn check(&self) -> Option<Self::ViolationId>;

    fn is_valid(&self) -> bool {
        self.check().is_none()
    }
}

/// Constant-time equality to prevent timing attacks.
pub trait ConstantTimeEq {
    fn ct_eq(&self, other: &Self) -> bool;
}

impl ConstantTimeEq for [u8; 32] {
    fn ct_eq(&self, other: &Self) -> bool {
        let mut acc: u8 = 0;
        for i in 0..32 {
            acc |= self[i] ^ other[i];
        }
        acc == 0
    }
}

impl ConstantTimeEq for H256 {
    fn ct_eq(&self, other: &Self) -> bool {
        self.0.ct_eq(&other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ct_eq_bytes() {
        let a = [0u8; 32];
        let b = [0u8; 32];
        let c = [1u8; 32];

        assert!(a.ct_eq(&b));
        assert!(!a.ct_eq(&c));
    }

    #[test]
    fn ct_eq_h256() {
        let a = H256::zero();
        let b = H256::zero();
        let c = H256::repeat_byte(0xff);

        assert!(a.ct_eq(&b));
        assert!(!a.ct_eq(&c));
    }
}
