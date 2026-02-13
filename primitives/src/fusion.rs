#![allow(clippy::result_unit_err)]

use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::Perbill;

#[cfg(feature = "std")]
use sp_core::blake2_256;

pub const DOMAIN_DEVICE_COMMITMENT: &[u8] = b"7ay:device:commit:v1";
pub const DOMAIN_DEVICE_REVEAL: &[u8] = b"7ay:device:reveal:v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct FusionWeights {
    pub heartbeat_weight: u8,
    pub device_weight: u8,
    pub position_weight: u8,
}

impl Default for FusionWeights {
    fn default() -> Self {
        Self {
            heartbeat_weight: 40,
            device_weight: 40,
            position_weight: 20,
        }
    }
}

impl FusionWeights {
    pub fn new(heartbeat: u8, device: u8, position: u8) -> Self {
        assert_eq!(
            heartbeat.saturating_add(device).saturating_add(position),
            100,
            "Fusion weights must sum to 100"
        );
        Self {
            heartbeat_weight: heartbeat,
            device_weight: device,
            position_weight: position,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.heartbeat_weight
            .saturating_add(self.device_weight)
            .saturating_add(self.position_weight)
            == 100
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct DeviceCommitment {
    pub commitment: H256,
    pub device_count: u8,
    pub timestamp: u64,
    pub block_number: u64,
}

impl DeviceCommitment {
    #[cfg(feature = "std")]
    pub fn new(
        device_mac_hashes: &[H256],
        nonce: &[u8; 32],
        block_number: u64,
        timestamp: u64,
    ) -> Self {
        let merkle_root = Self::compute_device_merkle_root(device_mac_hashes);
        let commitment = Self::compute_commitment(&merkle_root, nonce, block_number);

        Self {
            commitment,
            device_count: device_mac_hashes.len().min(255) as u8,
            timestamp,
            block_number,
        }
    }

    #[cfg(feature = "std")]
    pub fn compute_device_merkle_root(device_mac_hashes: &[H256]) -> H256 {
        if device_mac_hashes.is_empty() {
            return H256::zero();
        }

        let mut sorted: Vec<H256> = device_mac_hashes.to_vec();
        sorted.sort();

        let mut layer: Vec<H256> = sorted;
        while layer.len() > 1 {
            let mut next_layer = Vec::new();
            for chunk in layer.chunks(2) {
                let left = chunk[0];
                let right = chunk.get(1).copied().unwrap_or(left);
                let combined = [left.as_bytes(), right.as_bytes()].concat();
                next_layer.push(H256(blake2_256(&combined)));
            }
            layer = next_layer;
        }

        layer[0]
    }

    #[cfg(feature = "std")]
    pub fn compute_commitment(merkle_root: &H256, nonce: &[u8; 32], block_number: u64) -> H256 {
        let data = [
            DOMAIN_DEVICE_COMMITMENT,
            merkle_root.as_bytes(),
            nonce,
            &block_number.to_le_bytes(),
        ]
        .concat();
        H256(blake2_256(&data))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct DeviceReveal {
    pub commitment_block: u64,
    pub nonce: [u8; 32],
    pub device_merkle_root: H256,
    pub rssi_values: Vec<i8>,
    pub revealed_count: u8,
}

impl DeviceReveal {
    #[cfg(feature = "std")]
    pub fn verify(&self, commitment: &DeviceCommitment) -> bool {
        let recomputed =
            DeviceCommitment::compute_commitment(&self.device_merkle_root, &self.nonce, self.commitment_block);

        if recomputed != commitment.commitment {
            return false;
        }

        if self.commitment_block != commitment.block_number {
            return false;
        }

        true
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct FusedHealthMetrics {
    pub blocks_since_heartbeat: u32,
    pub consecutive_misses: u8,
    pub heartbeat_score: u8,
    pub device_observation_count: u32,
    pub device_consistency_score: u8,
    pub last_commitment: Option<H256>,
    pub successful_reveals: u32,
    pub position_variance: u32,
    pub triangulation_confirmations: u32,
    pub position_score: u8,
    pub fused_score: u8,
}

impl FusedHealthMetrics {
    pub fn calculate_fused_score(&mut self, weights: &FusionWeights) {
        let heartbeat_component =
            (self.heartbeat_score as u32 * weights.heartbeat_weight as u32) / 100;
        let device_component =
            (self.device_consistency_score as u32 * weights.device_weight as u32) / 100;
        let position_component =
            (self.position_score as u32 * weights.position_weight as u32) / 100;

        self.fused_score = (heartbeat_component + device_component + position_component)
            .min(100) as u8;
    }

    pub fn update_heartbeat(&mut self, decay_rate: u8, recovery_rate: u8, missed: bool) {
        if missed {
            self.consecutive_misses = self.consecutive_misses.saturating_add(1);
            self.heartbeat_score = self.heartbeat_score.saturating_sub(decay_rate);
        } else {
            self.consecutive_misses = 0;
            self.heartbeat_score = self.heartbeat_score.saturating_add(recovery_rate).min(100);
        }
    }

    pub fn update_device_metrics(
        &mut self,
        observed_count: u32,
        max_devices_for_full_score: u32,
        consistency_decay: u8,
    ) {
        self.device_observation_count = self
            .device_observation_count
            .saturating_add(observed_count);

        let base_score = if max_devices_for_full_score > 0 {
            ((observed_count * 100) / max_devices_for_full_score).min(100) as u8
        } else {
            0
        };

        if observed_count > 0 {
            self.device_consistency_score = self
                .device_consistency_score
                .saturating_add(5)
                .min(100);
        } else {
            self.device_consistency_score = self
                .device_consistency_score
                .saturating_sub(consistency_decay);
        }

        let _ = base_score;
    }

    pub fn update_position_metrics(&mut self, variance: u32, max_variance: u32, confirmed: bool) {
        self.position_variance = variance;

        if confirmed {
            self.triangulation_confirmations = self.triangulation_confirmations.saturating_add(1);
        }

        if max_variance > 0 && variance <= max_variance {
            self.position_score = (100 - (variance * 100 / max_variance).min(100)) as u8;
        } else {
            self.position_score = 0;
        }
    }

    pub fn is_critical(&self, threshold: u8) -> bool {
        self.fused_score < threshold
    }

    pub fn is_warning(&self, threshold: u8) -> bool {
        self.fused_score < threshold && !self.is_critical(threshold / 2)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum HealingReason {
    HeartbeatFailure,
    DeviceObservationFailure,
    PositionInconsistency,
    CriticalFusedHealth,
    CommitmentRevealMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct FusionConfig {
    pub weights: FusionWeights,
    pub commit_reveal_delay: u32,
    pub max_pending_commitments: u32,
    pub critical_threshold: u8,
    pub warning_threshold: u8,
    pub min_triangulation_nodes: u32,
    pub position_tolerance_meters: u32,
    pub max_devices_for_full_score: u32,
    pub consistency_decay_factor: u8,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            weights: FusionWeights::default(),
            commit_reveal_delay: 3,
            max_pending_commitments: 10,
            critical_threshold: 20,
            warning_threshold: 50,
            min_triangulation_nodes: 3,
            position_tolerance_meters: 50,
            max_devices_for_full_score: 10,
            consistency_decay_factor: 5,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct NodeObservation {
    pub node_id: u64,
    pub node_position: Position,
    pub rssi: i8,
    pub block_number: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Position {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn distance_squared(&self, other: &Position) -> u64 {
        let dx = (self.x as i64 - other.x as i64).abs() as u64;
        let dy = (self.y as i64 - other.y as i64).abs() as u64;
        let dz = (self.z as i64 - other.z as i64).abs() as u64;
        dx * dx + dy * dy + dz * dz
    }

    pub fn within_tolerance(&self, other: &Position, tolerance_meters: u32) -> bool {
        let tolerance_squared = (tolerance_meters as u64) * (tolerance_meters as u64);
        self.distance_squared(other) <= tolerance_squared
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct TriangulationProof {
    pub device_hash: H256,
    pub observations: Vec<NodeObservation>,
    pub calculated_position: Position,
    pub confidence: Perbill,
}

impl TriangulationProof {
    pub fn verify(&self, min_nodes: u32, tolerance_meters: u32) -> bool {
        if (self.observations.len() as u32) < min_nodes {
            return false;
        }

        if let Some(calculated) = self.calculate_position() {
            calculated.within_tolerance(&self.calculated_position, tolerance_meters)
        } else {
            false
        }
    }

    pub fn calculate_position(&self) -> Option<Position> {
        if self.observations.is_empty() {
            return None;
        }

        let total_weight: u64 = self
            .observations
            .iter()
            .map(|o| rssi_to_weight(o.rssi))
            .sum();

        if total_weight == 0 {
            return None;
        }

        let weighted_x: i64 = self
            .observations
            .iter()
            .map(|o| (o.node_position.x as i64) * (rssi_to_weight(o.rssi) as i64))
            .sum();

        let weighted_y: i64 = self
            .observations
            .iter()
            .map(|o| (o.node_position.y as i64) * (rssi_to_weight(o.rssi) as i64))
            .sum();

        let weighted_z: i64 = self
            .observations
            .iter()
            .map(|o| (o.node_position.z as i64) * (rssi_to_weight(o.rssi) as i64))
            .sum();

        Some(Position {
            x: (weighted_x / total_weight as i64) as i32,
            y: (weighted_y / total_weight as i64) as i32,
            z: (weighted_z / total_weight as i64) as i32,
        })
    }
}

fn rssi_to_weight(rssi: i8) -> u64 {
    let normalized = (rssi.saturating_add(100)).max(0) as u64;
    normalized * normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fusion_weights_default() {
        let weights = FusionWeights::default();
        assert!(weights.is_valid());
        assert_eq!(weights.heartbeat_weight, 40);
        assert_eq!(weights.device_weight, 40);
        assert_eq!(weights.position_weight, 20);
    }

    #[test]
    fn test_fusion_weights_custom() {
        let weights = FusionWeights::new(50, 30, 20);
        assert!(weights.is_valid());
    }

    #[test]
    #[should_panic(expected = "Fusion weights must sum to 100")]
    fn test_fusion_weights_invalid() {
        let _ = FusionWeights::new(50, 50, 50);
    }

    #[test]
    fn test_fused_health_calculation() {
        let mut metrics = FusedHealthMetrics {
            heartbeat_score: 100,
            device_consistency_score: 80,
            position_score: 60,
            ..Default::default()
        };

        let weights = FusionWeights::default();
        metrics.calculate_fused_score(&weights);
        assert_eq!(metrics.fused_score, 84);
    }

    #[test]
    fn test_position_distance() {
        let p1 = Position::new(0, 0, 0);
        let p2 = Position::new(3, 4, 0);
        assert_eq!(p1.distance_squared(&p2), 25);
        assert!(p1.within_tolerance(&p2, 5));
        assert!(!p1.within_tolerance(&p2, 4));
    }

    #[test]
    fn test_rssi_to_weight() {
        let strong = rssi_to_weight(-30);
        let weak = rssi_to_weight(-80);
        assert!(strong > weak);
        assert_eq!(rssi_to_weight(-100), 0);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_device_commitment_creation() {
        let devices = vec![
            H256::repeat_byte(0x01),
            H256::repeat_byte(0x02),
            H256::repeat_byte(0x03),
        ];
        let nonce = [0u8; 32];
        let block = 100;
        let timestamp = 1234567890;

        let commitment = DeviceCommitment::new(&devices, &nonce, block, timestamp);

        assert_eq!(commitment.device_count, 3);
        assert_eq!(commitment.block_number, block);
        assert_eq!(commitment.timestamp, timestamp);
        assert_ne!(commitment.commitment, H256::zero());
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_device_reveal_verification() {
        let devices = vec![H256::repeat_byte(0x01), H256::repeat_byte(0x02)];
        let nonce = [42u8; 32];
        let block = 100;
        let timestamp = 1234567890;

        let commitment = DeviceCommitment::new(&devices, &nonce, block, timestamp);

        let reveal = DeviceReveal {
            commitment_block: block,
            nonce,
            device_merkle_root: DeviceCommitment::compute_device_merkle_root(&devices),
            rssi_values: vec![-50, -60],
            revealed_count: 2,
        };

        assert!(reveal.verify(&commitment));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_device_reveal_wrong_nonce() {
        let devices = vec![H256::repeat_byte(0x01)];
        let nonce = [42u8; 32];
        let wrong_nonce = [43u8; 32];
        let block = 100;

        let commitment = DeviceCommitment::new(&devices, &nonce, block, 0);

        let reveal = DeviceReveal {
            commitment_block: block,
            nonce: wrong_nonce,
            device_merkle_root: DeviceCommitment::compute_device_merkle_root(&devices),
            rssi_values: vec![-50],
            revealed_count: 1,
        };

        assert!(!reveal.verify(&commitment));
    }
}
