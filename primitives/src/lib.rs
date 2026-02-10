//! Core primitives for the 7ay Proof of Presence Protocol.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

extern crate alloc;

pub mod constants;
pub mod crypto;
pub mod errors;
pub mod traits;
pub mod types;

pub use constants::*;
pub use errors::{ProtocolError, ProtocolResult};
pub use types::*;

// Re-export crypto with explicit names to avoid conflicts
pub use crypto::{
    hash_pair, hash_with_domain, Commitment as CryptoCommitment, MerkleProof, Nullifier,
    PresenceProof, PresenceStatement, PresenceWitness, Share, ShareIndex, StateRoot,
    DOMAIN_COMMITMENT, DOMAIN_EPOCH, DOMAIN_MERKLE, DOMAIN_NULLIFIER, DOMAIN_PRESENCE,
};

// Re-export traits with explicit names
pub use traits::{
    AggregateSignature, ChainBound, Commitment, ConstantTimeEq, CryptoHash, DomainSeparatedHash,
    EpochBound, Invariant, MerkleTree, SecretSharing, Signature, StateTransition, ZkProof,
};
