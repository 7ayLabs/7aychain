//! Core domain types for the 7ay Proof of Presence Protocol.

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::RuntimeDebug;

use crate::traits::EpochBound;

// =============================================================================
// Identity Types
// =============================================================================

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
    Default,
)]
pub struct ActorId(pub H256);

impl ActorId {
    pub const fn from_raw(bytes: [u8; 32]) -> Self {
        Self(H256(bytes))
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0 .0
    }
}

impl From<H256> for ActorId {
    fn from(hash: H256) -> Self {
        Self(hash)
    }
}

impl From<[u8; 32]> for ActorId {
    fn from(bytes: [u8; 32]) -> Self {
        Self(H256(bytes))
    }
}

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
    Default,
)]
pub struct ValidatorId(pub H256);

impl ValidatorId {
    pub const fn from_raw(bytes: [u8; 32]) -> Self {
        Self(H256(bytes))
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0 .0
    }
}

impl From<H256> for ValidatorId {
    fn from(hash: H256) -> Self {
        Self(hash)
    }
}

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
    Default,
)]
pub struct EpochId(pub u64);

impl EpochId {
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    pub const fn checked_sub(self, n: u64) -> Option<Self> {
        match self.0.checked_sub(n) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    pub const fn inner(self) -> u64 {
        self.0
    }
}

impl From<u64> for EpochId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

// =============================================================================
// Presence State Machine (INV1-13)
// =============================================================================

/// State: None -> Declared -> Validated -> Finalized | Slashed
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
pub enum PresenceState {
    #[default]
    None,
    Declared,
    Validated,
    Finalized,
    Slashed,
}

impl PresenceState {
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Finalized | Self::Slashed)
    }

    /// INV7: Monotonic forward-only transitions
    pub const fn can_transition_to(&self, target: &Self) -> bool {
        matches!(
            (self, target),
            (Self::None, Self::Declared)
                | (Self::Declared, Self::Validated)
                | (Self::Validated, Self::Finalized)
                | (Self::None | Self::Declared | Self::Validated, Self::Slashed)
        )
    }
}

// =============================================================================
// Epoch State Machine
// =============================================================================

/// State: Scheduled -> Active -> Closed -> Finalized
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
pub enum EpochState {
    #[default]
    Scheduled,
    Active,
    Closed,
    Finalized,
}

impl EpochState {
    pub const fn allows_declarations(&self) -> bool {
        matches!(self, Self::Active)
    }

    pub const fn allows_validations(&self) -> bool {
        matches!(self, Self::Active)
    }

    pub const fn can_transition_to(&self, target: &Self) -> bool {
        matches!(
            (self, target),
            (Self::Scheduled, Self::Active)
                | (Self::Active, Self::Closed)
                | (Self::Closed, Self::Finalized)
        )
    }
}

// =============================================================================
// Records
// =============================================================================

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
pub struct PresenceRecord<BlockNumber> {
    pub actor: ActorId,
    pub epoch: EpochId,
    pub state: PresenceState,
    pub declared_at: Option<BlockNumber>,
    pub validated_at: Option<BlockNumber>,
    pub finalized_at: Option<BlockNumber>,
    pub vote_count: u32,
}

impl<BlockNumber: Default> Default for PresenceRecord<BlockNumber> {
    fn default() -> Self {
        Self {
            actor: ActorId::default(),
            epoch: EpochId::default(),
            state: PresenceState::None,
            declared_at: None,
            validated_at: None,
            finalized_at: None,
            vote_count: 0,
        }
    }
}

impl<BlockNumber> PresenceRecord<BlockNumber> {
    pub const fn new(actor: ActorId, epoch: EpochId) -> Self {
        Self {
            actor,
            epoch,
            state: PresenceState::None,
            declared_at: None,
            validated_at: None,
            finalized_at: None,
            vote_count: 0,
        }
    }
}

impl<BlockNumber> EpochBound for PresenceRecord<BlockNumber> {
    type EpochId = EpochId;

    fn epoch(&self) -> Self::EpochId {
        self.epoch
    }
}

// =============================================================================
// Validator Types
// =============================================================================

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
pub enum ValidatorStatus {
    #[default]
    Inactive,
    Active,
    Recovering,
    Suspended,
}

impl ValidatorStatus {
    pub const fn can_vote(&self) -> bool {
        matches!(self, Self::Active)
    }
}

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
)]
pub enum ViolationType {
    Minor,    // 5%
    Moderate, // 20%
    Severe,   // 50%
    Critical, // 100%
}

impl ViolationType {
    pub const fn slash_percent(&self) -> u8 {
        match self {
            Self::Minor => 5,
            Self::Moderate => 20,
            Self::Severe => 50,
            Self::Critical => 100,
        }
    }
}

// =============================================================================
// Quorum
// =============================================================================

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
)]
pub struct QuorumConfig {
    pub threshold: u32,
    pub total: u32,
}

impl QuorumConfig {
    pub const fn new(threshold: u32, total: u32) -> Self {
        Self { threshold, total }
    }

    /// INV10: Validated requires votes >= threshold
    pub const fn is_met(&self, votes: u32) -> bool {
        votes >= self.threshold
    }

    pub const fn is_valid(&self) -> bool {
        self.threshold <= self.total && self.total > 0
    }
}

impl Default for QuorumConfig {
    fn default() -> Self {
        Self {
            threshold: 3,
            total: 5,
        }
    }
}

// =============================================================================
// Block Reference (INV43: Chain Binding)
// =============================================================================

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
)]
pub struct BlockRef {
    pub number: u64,
    pub hash: H256,
}

impl BlockRef {
    pub const fn new(number: u64, hash: H256) -> Self {
        Self { number, hash }
    }
}

// =============================================================================
// Vote
// =============================================================================

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
pub struct Vote {
    pub validator: ValidatorId,
    pub actor: ActorId,
    pub epoch: EpochId,
    pub block_ref: BlockRef,
    pub approve: bool,
}

impl EpochBound for Vote {
    type EpochId = EpochId;

    fn epoch(&self) -> Self::EpochId {
        self.epoch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_transitions() {
        assert!(PresenceState::None.can_transition_to(&PresenceState::Declared));
        assert!(PresenceState::Declared.can_transition_to(&PresenceState::Validated));
        assert!(PresenceState::Validated.can_transition_to(&PresenceState::Finalized));
        assert!(PresenceState::None.can_transition_to(&PresenceState::Slashed));
        assert!(PresenceState::Declared.can_transition_to(&PresenceState::Slashed));
        assert!(PresenceState::Validated.can_transition_to(&PresenceState::Slashed));

        assert!(!PresenceState::Finalized.can_transition_to(&PresenceState::Slashed));
        assert!(!PresenceState::Slashed.can_transition_to(&PresenceState::Finalized));
        assert!(!PresenceState::Declared.can_transition_to(&PresenceState::None));
    }

    #[test]
    fn epoch_transitions() {
        assert!(EpochState::Scheduled.can_transition_to(&EpochState::Active));
        assert!(EpochState::Active.can_transition_to(&EpochState::Closed));
        assert!(EpochState::Closed.can_transition_to(&EpochState::Finalized));

        assert!(!EpochState::Active.can_transition_to(&EpochState::Scheduled));
        assert!(!EpochState::Finalized.can_transition_to(&EpochState::Active));
    }

    #[test]
    fn epoch_id_arithmetic() {
        let epoch = EpochId::new(5);
        assert_eq!(epoch.next().inner(), 6);
        assert_eq!(epoch.checked_sub(3), Some(EpochId::new(2)));
        assert_eq!(EpochId::new(0).checked_sub(1), None);

        let max = EpochId::new(u64::MAX);
        assert_eq!(max.next().inner(), u64::MAX);
    }

    #[test]
    fn quorum() {
        let config = QuorumConfig::new(3, 5);
        assert!(config.is_valid());
        assert!(!config.is_met(2));
        assert!(config.is_met(3));
        assert!(config.is_met(5));

        assert!(!QuorumConfig::new(6, 5).is_valid());
    }

    #[test]
    fn violation_slash() {
        assert_eq!(ViolationType::Minor.slash_percent(), 5);
        assert_eq!(ViolationType::Moderate.slash_percent(), 20);
        assert_eq!(ViolationType::Severe.slash_percent(), 50);
        assert_eq!(ViolationType::Critical.slash_percent(), 100);
    }
}
