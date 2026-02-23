//! Witness-based presence triangulation types.
//!
//! This module implements the Presence-Based Triangulation (PBT) protocol
//! where nodes verify each other's presence through network measurements
//! rather than external device scanning (WiFi/Bluetooth).

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;

use crate::fusion::Position;
use crate::types::{ActorId, EpochId, ValidatorId};

/// Speed of light approximation for network latency calculations.
/// Practical network speed is ~150km per millisecond (half of theoretical).
pub const NETWORK_SPEED_KM_PER_MS: u32 = 150;

/// Minimum witnesses required for triangulation.
pub const MIN_WITNESSES_FOR_TRIANGULATION: u32 = 3;

/// Maximum latency considered valid (prevents gaming with artificial delays).
pub const MAX_VALID_LATENCY_MS: u32 = 1000;

// =============================================================================
// Scanner Types (Replaces WiFi/Bluetooth)
// =============================================================================

/// Scanner types for presence verification without hardware dependencies.
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
pub enum ScannerType {
    /// Network round-trip time measurement
    #[default]
    NetworkLatency,
    /// P2P connection topology (direct vs relayed)
    PeerTopology,
    /// Block propagation timing
    BlockPropagation,
    /// IP-based geolocation (approximate)
    IPGeolocation,
    /// User-provided GPS coordinates (opt-in)
    GPSConsent,
    /// Consensus-based witness attestation
    ConsensusWitness,
}

impl ScannerType {
    /// Whether this scanner type requires special hardware.
    pub const fn requires_hardware(&self) -> bool {
        matches!(self, Self::GPSConsent)
    }

    /// Whether this scanner type is privacy-preserving.
    pub const fn is_privacy_safe(&self) -> bool {
        // All types are privacy-safe since they only measure network properties
        // or use self-declared data with consent
        true
    }
}

// =============================================================================
// Latency Measurement
// =============================================================================

/// A network latency measurement between two nodes.
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
pub struct LatencyMeasurement {
    /// Round-trip time in milliseconds
    pub rtt_ms: u32,
    /// Whether this is a direct P2P connection
    pub direct_connection: bool,
    /// Number of network hops (if known)
    pub hops: Option<u8>,
    /// Measurement timestamp (block number)
    pub measured_at: u64,
}

impl LatencyMeasurement {
    /// Create a new latency measurement.
    pub fn new(rtt_ms: u32, direct: bool, measured_at: u64) -> Self {
        Self {
            rtt_ms,
            direct_connection: direct,
            hops: None,
            measured_at,
        }
    }

    /// Calculate maximum possible distance based on latency.
    /// Uses speed of light with practical network overhead.
    pub fn max_distance_km(&self) -> u32 {
        // RTT / 2 for one-way, then multiply by network speed
        (self.rtt_ms / 2) * NETWORK_SPEED_KM_PER_MS
    }

    /// Check if the measurement is within valid bounds.
    pub fn is_valid(&self) -> bool {
        self.rtt_ms > 0 && self.rtt_ms <= MAX_VALID_LATENCY_MS
    }
}

// =============================================================================
// Witness Attestation
// =============================================================================

/// An attestation from one node witnessing another's presence.
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
pub struct WitnessAttestation<BlockNumber> {
    /// The validator/node providing the attestation
    pub witness: ValidatorId,
    /// The actor being witnessed
    pub target: ActorId,
    /// The epoch of the attestation
    pub epoch: EpochId,
    /// Network latency measurement
    pub latency: LatencyMeasurement,
    /// Witness's known position
    pub witness_position: Position,
    /// Maximum distance from witness (calculated from latency)
    pub max_distance_km: u32,
    /// Block when attestation was made
    pub attested_at: BlockNumber,
    /// Scanner type used for measurement
    pub scanner_type: ScannerType,
}

impl<BlockNumber: Clone> WitnessAttestation<BlockNumber> {
    /// Create a new witness attestation.
    pub fn new(
        witness: ValidatorId,
        target: ActorId,
        epoch: EpochId,
        latency: LatencyMeasurement,
        witness_position: Position,
        attested_at: BlockNumber,
    ) -> Self {
        let max_distance_km = latency.max_distance_km();
        Self {
            witness,
            target,
            epoch,
            latency,
            witness_position,
            max_distance_km,
            attested_at,
            scanner_type: ScannerType::NetworkLatency,
        }
    }

    /// Check if a claimed position is within the attestation bounds.
    pub fn is_position_valid(&self, claimed: &Position) -> bool {
        // Convert km to meters for position comparison
        let max_distance_m = self.max_distance_km * 1000;
        let distance_sq = self.witness_position.distance_squared(claimed);
        let max_distance_sq = (max_distance_m as u64) * (max_distance_m as u64);
        distance_sq <= max_distance_sq
    }
}

// =============================================================================
// Position Claim
// =============================================================================

/// A node's claim about their physical position.
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
pub struct PositionClaim<BlockNumber> {
    /// The actor making the claim
    pub actor: ActorId,
    /// Claimed position
    pub claimed_position: Position,
    /// Epoch of the claim
    pub epoch: EpochId,
    /// Block when claimed
    pub claimed_at: BlockNumber,
    /// Number of witnesses who have attested
    pub witness_count: u32,
    /// Triangulated position (calculated from witnesses)
    pub triangulated_position: Option<Position>,
    /// Confidence score (0-100)
    pub confidence: u8,
    /// Whether the claim has been verified
    pub verified: bool,
}

impl<BlockNumber: Default + Clone> PositionClaim<BlockNumber> {
    /// Create a new position claim.
    pub fn new(
        actor: ActorId,
        position: Position,
        epoch: EpochId,
        claimed_at: BlockNumber,
    ) -> Self {
        Self {
            actor,
            claimed_position: position,
            epoch,
            claimed_at,
            witness_count: 0,
            triangulated_position: None,
            confidence: 0,
            verified: false,
        }
    }

    /// Update with triangulation result.
    pub fn update_triangulation(&mut self, position: Position, confidence: u8) {
        self.triangulated_position = Some(position);
        self.confidence = confidence;
    }

    /// Check if claimed position matches triangulated position within tolerance.
    pub fn is_consistent(&self, tolerance_meters: u32) -> bool {
        match &self.triangulated_position {
            Some(triangulated) => self
                .claimed_position
                .within_tolerance(triangulated, tolerance_meters),
            None => false,
        }
    }
}

// =============================================================================
// Triangulation Result
// =============================================================================

/// Result of triangulating a position from multiple witnesses.
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
pub struct TriangulationResult {
    /// Calculated position
    pub position: Position,
    /// Confidence score (0-100)
    pub confidence: u8,
    /// Number of witnesses used
    pub witness_count: u32,
    /// Whether the result is considered reliable
    pub reliable: bool,
}

impl TriangulationResult {
    /// Minimum witnesses for reliable triangulation.
    pub const MIN_WITNESSES: u32 = MIN_WITNESSES_FOR_TRIANGULATION;

    /// Create a new triangulation result.
    pub fn new(position: Position, witness_count: u32) -> Self {
        let reliable = witness_count >= Self::MIN_WITNESSES;
        let confidence = Self::calculate_confidence(witness_count);
        Self {
            position,
            confidence,
            witness_count,
            reliable,
        }
    }

    /// Calculate confidence based on witness count.
    fn calculate_confidence(witness_count: u32) -> u8 {
        match witness_count {
            0 => 0,
            1 => 20,
            2 => 40,
            3 => 60,
            4 => 75,
            5 => 85,
            6..=10 => 90,
            _ => 95,
        }
    }
}

// =============================================================================
// Witness Circle (for geometric triangulation)
// =============================================================================

/// A circle defined by a witness position and max distance.
/// Used for geometric intersection in triangulation.
#[derive(Clone, Copy, Debug)]
pub struct WitnessCircle {
    /// Center of the circle (witness position)
    pub center: Position,
    /// Radius in meters
    pub radius_m: u32,
}

impl WitnessCircle {
    /// Create from a witness attestation.
    pub fn from_attestation<BlockNumber>(attestation: &WitnessAttestation<BlockNumber>) -> Self {
        Self {
            center: attestation.witness_position,
            radius_m: attestation.max_distance_km * 1000,
        }
    }

    /// Check if a point is inside this circle.
    pub fn contains(&self, point: &Position) -> bool {
        let distance_sq = self.center.distance_squared(point);
        let radius_sq = (self.radius_m as u64) * (self.radius_m as u64);
        distance_sq <= radius_sq
    }
}

/// Triangulate position from multiple witness circles.
/// Uses weighted centroid of circle intersection region.
pub fn triangulate_from_witnesses<BlockNumber: Clone>(
    attestations: &[WitnessAttestation<BlockNumber>],
) -> Option<TriangulationResult> {
    if attestations.is_empty() {
        return None;
    }

    // Calculate weighted centroid based on inverse of max_distance
    // Closer witnesses (smaller circles) have more weight
    let mut total_weight: u64 = 0;
    let mut weighted_x: i64 = 0;
    let mut weighted_y: i64 = 0;
    let mut weighted_z: i64 = 0;

    for attestation in attestations {
        // Weight is inverse of distance (closer = more weight)
        // Add 1 to avoid division by zero; clamp to min 1 so every valid attestation contributes
        let weight = (1000u64 / (attestation.max_distance_km as u64 + 1)).max(1);

        weighted_x += (attestation.witness_position.x as i64) * (weight as i64);
        weighted_y += (attestation.witness_position.y as i64) * (weight as i64);
        weighted_z += (attestation.witness_position.z as i64) * (weight as i64);
        total_weight += weight;
    }

    if total_weight == 0 {
        return None;
    }

    let position = Position {
        x: (weighted_x / total_weight as i64) as i32,
        y: (weighted_y / total_weight as i64) as i32,
        z: (weighted_z / total_weight as i64) as i32,
    };

    Some(TriangulationResult::new(
        position,
        attestations.len() as u32,
    ))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_to_distance() {
        let measurement = LatencyMeasurement::new(10, true, 100);
        // 10ms RTT -> 5ms one-way -> 5 * 150 = 750km
        assert_eq!(measurement.max_distance_km(), 750);
    }

    #[test]
    fn test_latency_validation() {
        let valid = LatencyMeasurement::new(100, true, 100);
        assert!(valid.is_valid());

        let zero = LatencyMeasurement::new(0, true, 100);
        assert!(!zero.is_valid());

        let too_high = LatencyMeasurement::new(2000, true, 100);
        assert!(!too_high.is_valid());
    }

    #[test]
    fn test_position_claim_consistency() {
        let mut claim = PositionClaim::new(
            ActorId::default(),
            Position::new(1000, 2000, 0),
            EpochId::new(1),
            0u64,
        );

        // No triangulation yet
        assert!(!claim.is_consistent(100));

        // Triangulation close to claim
        claim.update_triangulation(Position::new(1050, 2050, 0), 80);
        assert!(claim.is_consistent(100)); // Within 100m

        // Triangulation far from claim
        claim.update_triangulation(Position::new(5000, 5000, 0), 80);
        assert!(!claim.is_consistent(100));
    }

    #[test]
    fn test_triangulation_confidence() {
        assert_eq!(TriangulationResult::calculate_confidence(0), 0);
        assert_eq!(TriangulationResult::calculate_confidence(1), 20);
        assert_eq!(TriangulationResult::calculate_confidence(3), 60);
        assert_eq!(TriangulationResult::calculate_confidence(5), 85);
        assert_eq!(TriangulationResult::calculate_confidence(10), 90);
    }

    #[test]
    fn test_witness_circle_contains() {
        let circle = WitnessCircle {
            center: Position::new(0, 0, 0),
            radius_m: 100,
        };

        assert!(circle.contains(&Position::new(50, 50, 0)));
        assert!(circle.contains(&Position::new(0, 0, 0)));
        assert!(!circle.contains(&Position::new(100, 100, 0))); // ~141m away
    }

    #[test]
    fn test_triangulate_from_witnesses() {
        let attestations: Vec<WitnessAttestation<u64>> = vec![
            WitnessAttestation::new(
                ValidatorId::default(),
                ActorId::default(),
                EpochId::new(1),
                LatencyMeasurement::new(10, true, 100),
                Position::new(0, 0, 0),
                100,
            ),
            WitnessAttestation::new(
                ValidatorId::default(),
                ActorId::default(),
                EpochId::new(1),
                LatencyMeasurement::new(10, true, 100),
                Position::new(1000, 0, 0),
                100,
            ),
            WitnessAttestation::new(
                ValidatorId::default(),
                ActorId::default(),
                EpochId::new(1),
                LatencyMeasurement::new(10, true, 100),
                Position::new(500, 866, 0), // Equilateral triangle
                100,
            ),
        ];

        let result = triangulate_from_witnesses(&attestations).unwrap();
        assert_eq!(result.witness_count, 3);
        assert!(result.reliable);
        // Centroid should be roughly at (500, 288, 0)
        assert!(result.position.x > 400 && result.position.x < 600);
    }

    #[test]
    fn test_scanner_type_properties() {
        assert!(!ScannerType::NetworkLatency.requires_hardware());
        assert!(ScannerType::GPSConsent.requires_hardware());
        assert!(ScannerType::NetworkLatency.is_privacy_safe());
        assert!(ScannerType::PeerTopology.is_privacy_safe());
    }
}
