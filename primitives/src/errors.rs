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
    UnauthorizedDeclaration, // INV4
    InvalidStateTransition,  // INV7
    SlashedTerminal,         // INV8
    EpochExpired,            // INV9
    QuorumNotMet,            // INV10
    DuplicateVote,           // INV11
    CommitmentNotRevealed,   // INV13

    // Epoch (INV14-18)
    EpochInvalidState,
    EpochNotFound,
    EphemeralOutOfBounds,     // INV14
    EpochSequenceGap,         // INV15
    ActorNotInEpoch,          // INV16
    EpochImmutable,           // INV17
    EpochTransitionGrace,     // INV18

    // Validator (INV46-49)
    InsufficientValidators, // INV46
    StakeConcentration,     // INV47
    ValidatorNotActive,
    InsufficientStake,
    ValidatorUnauthorized,
    InvalidQuorumConfig,

    // Dispute
    DisputeTargetNotFound,
    DisputeTargetNotValidator,
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
    BoomerangPathInvalid,      // INV30
    BoomerangTimeout,          // INV31
    BoomerangExtensionLimit,   // INV32
    BoomerangVerification,     // INV33

    // Autonomous (INV34-37)
    AutonomousIntentInvalid, // INV34
    AutonomousThreshold,     // INV35
    AutonomousPatternLimit,  // INV36
    AutonomousExpired,       // INV37

    // Octopus (INV38-42, INV63)
    OctopusPremature,         // INV38
    OctopusActivationInvalid, // INV39
    OctopusScalingInvalid,    // INV40
    OctopusStateInconsistent, // INV41
    OctopusClusterInvalid,    // INV42
    OctopusSubnodeLimit,      // INV63

    // Small Network (INV54-56)
    SmallNetworkQuorum,       // INV54
    SmallNetworkVerification, // INV55
    SmallNetworkDiscovery,    // INV56

    // Reputation (INV50-53)
    ReputationOutOfBounds,    // INV50
    ReputationUpdateInvalid,  // INV51
    ReputationDecayInvalid,   // INV52
    CooldownActive,           // INV53

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
            // Presence
            Self::DuplicatePresence => Some("INV1"),
            Self::PresenceImmutable => Some("INV2"),
            Self::UnauthorizedDeclaration => Some("INV4"),
            Self::InvalidStateTransition => Some("INV7"),
            Self::SlashedTerminal => Some("INV8"),
            Self::EpochExpired => Some("INV9"),
            Self::QuorumNotMet => Some("INV10"),
            Self::DuplicateVote => Some("INV11"),
            Self::CommitmentNotRevealed => Some("INV13"),

            // Epoch
            Self::EphemeralOutOfBounds => Some("INV14"),
            Self::EpochSequenceGap => Some("INV15"),
            Self::ActorNotInEpoch => Some("INV16"),
            Self::EpochImmutable => Some("INV17"),
            Self::EpochTransitionGrace => Some("INV18"),

            // Boomerang
            Self::BoomerangPathInvalid => Some("INV30"),
            Self::BoomerangTimeout => Some("INV31"),
            Self::BoomerangExtensionLimit => Some("INV32"),
            Self::BoomerangVerification => Some("INV33"),

            // Autonomous
            Self::AutonomousIntentInvalid => Some("INV34"),
            Self::AutonomousThreshold => Some("INV35"),
            Self::AutonomousPatternLimit => Some("INV36"),
            Self::AutonomousExpired => Some("INV37"),

            // Octopus
            Self::OctopusPremature => Some("INV38"),
            Self::OctopusActivationInvalid => Some("INV39"),
            Self::OctopusScalingInvalid => Some("INV40"),
            Self::OctopusStateInconsistent => Some("INV41"),
            Self::OctopusClusterInvalid => Some("INV42"),
            Self::OctopusSubnodeLimit => Some("INV63"),

            // Security
            Self::ChainBindingInvalid | Self::BlockRefOutOfBounds => Some("INV43"),
            Self::KeyDestructionTimeout | Self::KeyDestructionAttestations => Some("INV44"),
            Self::DiscoveryRateLimit => Some("INV45"),

            // Validator Economics
            Self::InsufficientValidators => Some("INV46"),
            Self::StakeConcentration => Some("INV47"),
            Self::SlashExceeded => Some("INV48"),
            Self::RewardExceeded => Some("INV49"),

            // Reputation
            Self::ReputationOutOfBounds => Some("INV50"),
            Self::ReputationUpdateInvalid => Some("INV51"),
            Self::ReputationDecayInvalid => Some("INV52"),
            Self::CooldownActive => Some("INV53"),

            // Small Network
            Self::SmallNetworkQuorum => Some("INV54"),
            Self::SmallNetworkVerification => Some("INV55"),
            Self::SmallNetworkDiscovery => Some("INV56"),

            // Recovery
            Self::RecoveryQuorumNotMet => Some("INV57"),
            Self::RecoveryCooldown => Some("INV58"),
            Self::UpgradeDelayRequired => Some("INV59"),
            Self::EmergencyQuorumNotMet => Some("INV60"),

            // Device
            Self::DeviceIdentityInvalid => Some("INV64"),
            Self::DevicePresenceInvalid => Some("INV65"),

            // Vault
            Self::VaultRingInvalid => Some("INV66"),
            Self::VaultLocked => Some("INV67"),
            Self::VaultKeyNotDestroyed => Some("INV68"),

            // Crypto
            Self::InvalidShareDistribution => Some("INV69"),

            // Storage
            Self::StorageEpochBinding => Some("INV70"),
            Self::StorageAccessDenied => Some("INV71"),
            Self::StorageIntegrity => Some("INV72"),

            // ZK
            Self::ZkShareProofInvalid => Some("INV73"),
            Self::ZkPresenceProofInvalid => Some("INV74"),
            Self::ZkAccessProofInvalid => Some("INV75"),

            // Lifecycle
            Self::AutoLockTriggered => Some("INV76"),
            Self::KeyDestructionFailed => Some("INV77"),
            Self::KeyRotationRequired => Some("INV78"),

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

    /// Verify that all documented invariants (INV1-78) from documented ranges
    /// have at least one ProtocolError variant mapping to them.
    #[test]
    fn invariant_coverage_completeness() {
        // All invariants that should be covered based on CLAUDE.md
        let expected_invariants = [
            "INV1", "INV2", "INV4", "INV7", "INV8", "INV9", "INV10", "INV11", "INV13",
            "INV14", "INV15", "INV16", "INV17", "INV18",
            "INV30", "INV31", "INV32", "INV33",
            "INV34", "INV35", "INV36", "INV37",
            "INV38", "INV39", "INV40", "INV41", "INV42",
            "INV43", "INV44", "INV45",
            "INV46", "INV47", "INV48", "INV49",
            "INV50", "INV51", "INV52", "INV53",
            "INV54", "INV55", "INV56",
            "INV57", "INV58", "INV59", "INV60",
            "INV63", "INV64", "INV65",
            "INV66", "INV67", "INV68", "INV69",
            "INV70", "INV71", "INV72",
            "INV73", "INV74", "INV75",
            "INV76", "INV77", "INV78",
        ];

        // Collect all invariants from the enum
        let all_variants: Vec<ProtocolError> = vec![
            ProtocolError::DuplicatePresence,
            ProtocolError::PresenceImmutable,
            ProtocolError::UnauthorizedDeclaration,
            ProtocolError::InvalidStateTransition,
            ProtocolError::SlashedTerminal,
            ProtocolError::EpochExpired,
            ProtocolError::QuorumNotMet,
            ProtocolError::DuplicateVote,
            ProtocolError::CommitmentNotRevealed,
            ProtocolError::EphemeralOutOfBounds,
            ProtocolError::EpochSequenceGap,
            ProtocolError::ActorNotInEpoch,
            ProtocolError::EpochImmutable,
            ProtocolError::EpochTransitionGrace,
            ProtocolError::BoomerangPathInvalid,
            ProtocolError::BoomerangTimeout,
            ProtocolError::BoomerangExtensionLimit,
            ProtocolError::BoomerangVerification,
            ProtocolError::AutonomousIntentInvalid,
            ProtocolError::AutonomousThreshold,
            ProtocolError::AutonomousPatternLimit,
            ProtocolError::AutonomousExpired,
            ProtocolError::OctopusPremature,
            ProtocolError::OctopusActivationInvalid,
            ProtocolError::OctopusScalingInvalid,
            ProtocolError::OctopusStateInconsistent,
            ProtocolError::OctopusClusterInvalid,
            ProtocolError::OctopusSubnodeLimit,
            ProtocolError::ChainBindingInvalid,
            ProtocolError::BlockRefOutOfBounds,
            ProtocolError::KeyDestructionTimeout,
            ProtocolError::KeyDestructionAttestations,
            ProtocolError::DiscoveryRateLimit,
            ProtocolError::InsufficientValidators,
            ProtocolError::StakeConcentration,
            ProtocolError::SlashExceeded,
            ProtocolError::RewardExceeded,
            ProtocolError::ReputationOutOfBounds,
            ProtocolError::ReputationUpdateInvalid,
            ProtocolError::ReputationDecayInvalid,
            ProtocolError::CooldownActive,
            ProtocolError::SmallNetworkQuorum,
            ProtocolError::SmallNetworkVerification,
            ProtocolError::SmallNetworkDiscovery,
            ProtocolError::RecoveryQuorumNotMet,
            ProtocolError::RecoveryCooldown,
            ProtocolError::UpgradeDelayRequired,
            ProtocolError::EmergencyQuorumNotMet,
            ProtocolError::DeviceIdentityInvalid,
            ProtocolError::DevicePresenceInvalid,
            ProtocolError::VaultRingInvalid,
            ProtocolError::VaultLocked,
            ProtocolError::VaultKeyNotDestroyed,
            ProtocolError::InvalidShareDistribution,
            ProtocolError::StorageEpochBinding,
            ProtocolError::StorageAccessDenied,
            ProtocolError::StorageIntegrity,
            ProtocolError::ZkShareProofInvalid,
            ProtocolError::ZkPresenceProofInvalid,
            ProtocolError::ZkAccessProofInvalid,
            ProtocolError::AutoLockTriggered,
            ProtocolError::KeyDestructionFailed,
            ProtocolError::KeyRotationRequired,
        ];

        let mut covered: Vec<&str> = all_variants
            .iter()
            .filter_map(|e| e.invariant())
            .collect();
        covered.sort();
        covered.dedup();

        for inv in &expected_invariants {
            assert!(
                covered.contains(inv),
                "Missing ProtocolError mapping for {inv}"
            );
        }
    }
}
