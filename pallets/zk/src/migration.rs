//! ZK proof system migration infrastructure.
//!
//! Manages the transition from hash-based stub verifiers to production
//! cryptographic verifiers (Groth16, PlonK, Halo2).
//!
//! # Migration Path
//!
//! ```text
//! Legacy (v0.8.x) -> Transitional (v0.8.20) -> SnarkOnly (v0.9.0+)
//! ```
//!
//! - **Legacy:** Hash-based stub verifiers. All proof types accepted.
//! - **Transitional:** Both stub and SNARK proofs accepted. New circuits
//!   must be registered. Existing proofs continue to work.
//! - **SnarkOnly:** Only SNARK proofs accepted. Stub verifiers disabled.
//!   Requires all circuits to be registered with valid VKs.

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// Proof system operating mode controlling the verification strategy.
///
/// Transitions are monotonic: Legacy -> Transitional -> SnarkOnly.
/// Downgrading is not permitted (enforced by `can_transition_to`).
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum ProofSystemMode {
    /// Hash-based stub verifiers (development / pre-v0.8.20).
    /// All proof types use SimpleHashVerifier / StubVerifier.
    #[default]
    Legacy,

    /// Both stub and SNARK proofs accepted (v0.8.20 migration period).
    /// New circuits can be registered. Existing hash proofs still work.
    /// SNARK proofs are preferred when a circuit is available.
    Transitional,

    /// Only SNARK proofs accepted (v0.9.0+ production).
    /// Stub verifiers are completely disabled.
    /// All proof types must have registered circuits.
    SnarkOnly,
}

impl ProofSystemMode {
    /// Check if a transition to the target mode is valid.
    /// Transitions must be sequential (no skipping modes).
    pub fn can_transition_to(self, target: Self) -> bool {
        matches!(
            (self, target),
            (Self::Legacy, Self::Transitional) | (Self::Transitional, Self::SnarkOnly)
        )
    }

    /// Whether stub/hash-based proofs are accepted in this mode.
    pub fn accepts_stub_proofs(self) -> bool {
        matches!(self, Self::Legacy | Self::Transitional)
    }

    /// Whether SNARK proofs are accepted in this mode.
    pub fn accepts_snark_proofs(self) -> bool {
        matches!(self, Self::Transitional | Self::SnarkOnly)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_ordering() {
        assert!(ProofSystemMode::Legacy < ProofSystemMode::Transitional);
        assert!(ProofSystemMode::Transitional < ProofSystemMode::SnarkOnly);
        assert!(ProofSystemMode::Legacy < ProofSystemMode::SnarkOnly);
    }

    #[test]
    fn valid_sequential_transitions() {
        assert!(ProofSystemMode::Legacy.can_transition_to(ProofSystemMode::Transitional));
        assert!(ProofSystemMode::Transitional.can_transition_to(ProofSystemMode::SnarkOnly));
    }

    #[test]
    fn skip_transition_rejected() {
        assert!(!ProofSystemMode::Legacy.can_transition_to(ProofSystemMode::SnarkOnly));
    }

    #[test]
    fn invalid_backward_transitions() {
        assert!(!ProofSystemMode::Transitional.can_transition_to(ProofSystemMode::Legacy));
        assert!(!ProofSystemMode::SnarkOnly.can_transition_to(ProofSystemMode::Transitional));
        assert!(!ProofSystemMode::SnarkOnly.can_transition_to(ProofSystemMode::Legacy));
    }

    #[test]
    fn same_mode_transition_invalid() {
        assert!(!ProofSystemMode::Legacy.can_transition_to(ProofSystemMode::Legacy));
        assert!(!ProofSystemMode::Transitional.can_transition_to(ProofSystemMode::Transitional));
        assert!(!ProofSystemMode::SnarkOnly.can_transition_to(ProofSystemMode::SnarkOnly));
    }

    #[test]
    fn stub_proof_acceptance() {
        assert!(ProofSystemMode::Legacy.accepts_stub_proofs());
        assert!(ProofSystemMode::Transitional.accepts_stub_proofs());
        assert!(!ProofSystemMode::SnarkOnly.accepts_stub_proofs());
    }

    #[test]
    fn snark_proof_acceptance() {
        assert!(!ProofSystemMode::Legacy.accepts_snark_proofs());
        assert!(ProofSystemMode::Transitional.accepts_snark_proofs());
        assert!(ProofSystemMode::SnarkOnly.accepts_snark_proofs());
    }

    #[test]
    fn default_is_legacy() {
        assert_eq!(ProofSystemMode::default(), ProofSystemMode::Legacy);
    }
}
