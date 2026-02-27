//! Core primitives for the 7ay Proof of Presence Protocol.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

extern crate alloc;

pub mod constants;
pub mod crypto;
pub mod errors;
pub mod fusion;
pub mod traits;
pub mod triangulation;
pub mod types;
pub mod witness;

pub use constants::*;
pub use errors::{ProtocolError, ProtocolResult};
pub use types::*;

// Re-export crypto with explicit names to avoid conflicts
pub use crypto::{
    derive_actor_id, derive_validator_id, hash_pair, hash_with_domain, MerkleProof, Nullifier,
    PresenceCommitment, PresenceProof, PresenceStatement, PresenceWitness, Share, ShareIndex,
    StateRoot, DOMAIN_ACTOR, DOMAIN_COMMITMENT, DOMAIN_EPOCH, DOMAIN_MERKLE, DOMAIN_NULLIFIER,
    DOMAIN_PRESENCE, DOMAIN_VALIDATOR_ID,
};

// Re-export traits with explicit names
pub use traits::{
    AggregateSignature, ChainBound, Commitment, ConstantTimeEq, CryptoHash, DomainSeparatedHash,
    EpochBound, EpochProvider, Invariant, MerkleTree, SecretSharing, Signature, StateTransition,
    ValidatorProvider, ZkProof,
};

pub use fusion::{
    DeviceCommitment, DeviceReveal, FusedHealthMetrics, FusionConfig, FusionWeights, HealingReason,
    NodeObservation, Position, TriangulationProof, DOMAIN_DEVICE_COMMITMENT, DOMAIN_DEVICE_REVEAL,
};

pub use triangulation::{
    calculate_weighted_centroid, multilateration, rssi_to_distance_cm, DeviceTrack,
    SignalObservation, TriangulatedPosition, TriangulationConfig, Velocity,
};

pub use witness::{
    triangulate_from_witnesses, LatencyMeasurement, PositionClaim, ScannerType,
    TriangulationResult, WitnessAttestation, WitnessCircle, MAX_VALID_LATENCY_MS,
    MIN_WITNESSES_FOR_TRIANGULATION, NETWORK_SPEED_KM_PER_MS,
};
