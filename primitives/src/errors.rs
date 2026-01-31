//! Protocol error types with invariant references.

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

pub type ProtocolResult<T> = Result<T, ProtocolError>;

#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, RuntimeDebug)]
pub enum ProtocolError {
    // Presence (INV1-13)
    DuplicatePresence,       // INV1
    PresenceImmutable,       // INV2
    InvalidStateTransition,  // INV7
    UnauthorizedDeclaration, // INV4
    EpochExpired,            // INV9
    QuorumNotMet,            // INV10
    DuplicateVote,           // INV11
    SlashedTerminal,         // INV8

    // Epoch (INV14-18)
    EpochInvalidState,
    EpochNotFound,
    EphemeralOutOfBounds, // INV14
    ActorNotInEpoch,      // INV16

    // Validator (INV46-49)
    InsufficientValidators, // INV46
    StakeConcentration,     // INV47
    ValidatorNotActive,
    InsufficientStake,
    ValidatorUnauthorized,
    InvalidQuorumConfig,

    // Dispute
    DisputeTargetNotFound,
    InvalidEvidence,
    DisputeResolved,
    DisputeTimeout,

    // Slashing (INV48-49)
    SlashExceeded,  // INV48
    RewardExceeded, // INV49

    // Recovery (INV57-60)
    RecoveryQuorumNotMet,  // INV57
    RecoveryCooldown,      // INV58
    UpgradeDelayRequired,  // INV59
    EmergencyQuorumNotMet, // INV60

    // Security (INV43-45)
    ChainBindingInvalid,        // INV43
    BlockRefOutOfBounds,        // INV43
    KeyDestructionTimeout,      // INV44
    KeyDestructionAttestations, // INV44
    DiscoveryRateLimit,         // INV45

    // Vault (INV66-68)
    VaultRingInvalid,     // INV66
    VaultLocked,          // INV67
    VaultKeyNotDestroyed, // INV68

    // Device (INV64-65)
    DeviceIdentityInvalid, // INV64
    DevicePresenceInvalid, // INV65

    // Crypto (INV69)
    InvalidShareDistribution, // INV69
    CryptoFailed,
    SignatureInvalid,

    // Storage (INV70-72)
    StorageEpochBinding, // INV70
    StorageAccessDenied, // INV71
    StorageIntegrity,    // INV72

    // ZK Proofs (INV73-75)
    ZkShareProofInvalid,    // INV73
    ZkPresenceProofInvalid, // INV74
    ZkAccessProofInvalid,   // INV75

    // Lifecycle (INV76-78)
    AutoLockTriggered,    // INV76
    KeyDestructionFailed, // INV77
    KeyRotationRequired,  // INV78

    // Boomerang (INV30-33)
    BoomerangPathInvalid,  // INV30
    BoomerangTimeout,      // INV31
    BoomerangVerification, // INV33

    // Autonomous (INV34-37)
    AutonomousIntentInvalid, // INV34
    AutonomousThreshold,     // INV35
    AutonomousExpired,       // INV37

    // Octopus (INV38-42, INV63)
    OctopusPremature,         // INV38
    OctopusSubnodeLimit,      // INV63
    OctopusStateInconsistent, // INV41

    // Small Network (INV54-56)
    SmallNetworkVerification, // INV55

    // Reputation (INV50-53)
    ReputationOutOfBounds, // INV50
    CooldownActive,        // INV53

    // Generic
    ArithmeticOverflow,
    InvalidInput,
    NotPermitted,
    NotFound,
    Internal,
}

impl ProtocolError {
    pub const fn invariant(&self) -> Option<&'static str> {
        match self {
            Self::DuplicatePresence => Some("INV1"),
            Self::PresenceImmutable => Some("INV2"),
            Self::InvalidStateTransition => Some("INV7"),
            Self::UnauthorizedDeclaration => Some("INV4"),
            Self::EpochExpired => Some("INV9"),
            Self::QuorumNotMet => Some("INV10"),
            Self::DuplicateVote => Some("INV11"),
            Self::SlashedTerminal => Some("INV8"),
            Self::EphemeralOutOfBounds => Some("INV14"),
            Self::ActorNotInEpoch => Some("INV16"),
            Self::InsufficientValidators => Some("INV46"),
            Self::StakeConcentration => Some("INV47"),
            Self::SlashExceeded => Some("INV48"),
            Self::RewardExceeded => Some("INV49"),
            Self::RecoveryQuorumNotMet => Some("INV57"),
            Self::RecoveryCooldown => Some("INV58"),
            Self::UpgradeDelayRequired => Some("INV59"),
            Self::EmergencyQuorumNotMet => Some("INV60"),
            Self::ChainBindingInvalid | Self::BlockRefOutOfBounds => Some("INV43"),
            Self::KeyDestructionTimeout | Self::KeyDestructionAttestations => Some("INV44"),
            Self::DiscoveryRateLimit => Some("INV45"),
            Self::VaultRingInvalid => Some("INV66"),
            Self::VaultLocked => Some("INV67"),
            Self::VaultKeyNotDestroyed => Some("INV68"),
            Self::DeviceIdentityInvalid => Some("INV64"),
            Self::DevicePresenceInvalid => Some("INV65"),
            Self::InvalidShareDistribution => Some("INV69"),
            Self::StorageEpochBinding => Some("INV70"),
            Self::StorageAccessDenied => Some("INV71"),
            Self::StorageIntegrity => Some("INV72"),
            Self::ZkShareProofInvalid => Some("INV73"),
            Self::ZkPresenceProofInvalid => Some("INV74"),
            Self::ZkAccessProofInvalid => Some("INV75"),
            Self::AutoLockTriggered => Some("INV76"),
            Self::KeyDestructionFailed => Some("INV77"),
            Self::KeyRotationRequired => Some("INV78"),
            Self::BoomerangPathInvalid => Some("INV30"),
            Self::BoomerangTimeout => Some("INV31"),
            Self::BoomerangVerification => Some("INV33"),
            Self::AutonomousIntentInvalid => Some("INV34"),
            Self::AutonomousThreshold => Some("INV35"),
            Self::AutonomousExpired => Some("INV37"),
            Self::OctopusPremature => Some("INV38"),
            Self::OctopusSubnodeLimit => Some("INV63"),
            Self::OctopusStateInconsistent => Some("INV41"),
            Self::SmallNetworkVerification => Some("INV55"),
            Self::ReputationOutOfBounds => Some("INV50"),
            Self::CooldownActive => Some("INV53"),
            _ => None,
        }
    }

    pub const fn is_security_violation(&self) -> bool {
        matches!(
            self,
            Self::ChainBindingInvalid
                | Self::BlockRefOutOfBounds
                | Self::SignatureInvalid
                | Self::ZkShareProofInvalid
                | Self::ZkPresenceProofInvalid
                | Self::ZkAccessProofInvalid
                | Self::BoomerangVerification
                | Self::StorageIntegrity
        )
    }

    pub const fn is_slashable(&self) -> bool {
        matches!(
            self,
            Self::StakeConcentration
                | Self::DuplicateVote
                | Self::InvalidEvidence
                | Self::OctopusStateInconsistent
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariant_mapping() {
        assert_eq!(ProtocolError::DuplicatePresence.invariant(), Some("INV1"));
        assert_eq!(
            ProtocolError::InsufficientValidators.invariant(),
            Some("INV46")
        );
        assert_eq!(ProtocolError::Internal.invariant(), None);
    }

    #[test]
    fn security_violation() {
        assert!(ProtocolError::ChainBindingInvalid.is_security_violation());
        assert!(ProtocolError::SignatureInvalid.is_security_violation());
        assert!(!ProtocolError::QuorumNotMet.is_security_violation());
    }

    #[test]
    fn slashable_offense() {
        assert!(ProtocolError::StakeConcentration.is_slashable());
        assert!(ProtocolError::DuplicateVote.is_slashable());
        assert!(!ProtocolError::NotFound.is_slashable());
    }
}
