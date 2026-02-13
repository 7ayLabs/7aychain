#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;

pub const ALPHA: u8 = 40;
pub const BETA: u8 = 40;
pub const GAMMA: u8 = 20;

pub const CRITICAL_HEALTH_THRESHOLD: u8 = 20;
pub const WARNING_HEALTH_THRESHOLD: u8 = 50;
pub const MIN_TRIANGULATION_NODES: u32 = 3;
pub const POSITION_TOLERANCE_CM: u32 = 5000;
pub const MAX_DEVICES_FOR_FULL_SCORE: u32 = 10;
pub const CONSISTENCY_DECAY_FACTOR: u8 = 5;

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Default,
)]
pub struct FusionWeights {
    pub heartbeat_weight: u8,
    pub device_weight: u8,
    pub position_weight: u8,
}

impl FusionWeights {
    pub fn new(alpha: u8, beta: u8, gamma: u8) -> Option<Self> {
        if alpha.saturating_add(beta).saturating_add(gamma) != 100 {
            return None;
        }
        Some(Self {
            heartbeat_weight: alpha,
            device_weight: beta,
            position_weight: gamma,
        })
    }

    pub fn default_weights() -> Self {
        Self {
            heartbeat_weight: ALPHA,
            device_weight: BETA,
            position_weight: GAMMA,
        }
    }
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Default,
)]
pub struct Position {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl Position {
    pub fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    pub fn distance_squared(&self, other: &Self) -> u128 {
        let dx = (self.x - other.x).unsigned_abs() as u128;
        let dy = (self.y - other.y).unsigned_abs() as u128;
        let dz = (self.z - other.z).unsigned_abs() as u128;
        dx * dx + dy * dy + dz * dz
    }

    pub fn within_tolerance(&self, other: &Self, tolerance_cm: u32) -> bool {
        let dist_sq = self.distance_squared(other);
        let tol_sq = (tolerance_cm as u128) * (tolerance_cm as u128);
        dist_sq <= tol_sq
    }
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Default,
)]
pub struct DeviceObservationMetrics {
    pub last_commitment: Option<H256>,
    pub last_reveal_block: u64,
    pub total_observations: u32,
    pub consistency_score: u8,
    pub average_device_count: u8,
}

impl DeviceObservationMetrics {
    pub fn record_observation(&mut self, device_count: u8, block: u64, commitment: H256) {
        let prev_avg = self.average_device_count as u32;
        let total = self.total_observations.saturating_add(1);
        let new_avg = (prev_avg.saturating_mul(self.total_observations)
            .saturating_add(device_count as u32)) / total;

        self.average_device_count = new_avg.min(255) as u8;
        self.total_observations = total;
        self.last_commitment = Some(commitment);
        self.last_reveal_block = block;

        self.update_consistency(device_count);
    }

    fn update_consistency(&mut self, device_count: u8) {
        let expected = self.average_device_count;
        let diff = if device_count > expected {
            device_count - expected
        } else {
            expected - device_count
        };

        if diff <= 2 {
            self.consistency_score = self.consistency_score.saturating_add(5).min(100);
        } else if diff <= 5 {
            self.consistency_score = self.consistency_score.saturating_sub(2);
        } else {
            self.consistency_score = self.consistency_score.saturating_sub(CONSISTENCY_DECAY_FACTOR);
        }
    }

    pub fn device_score(&self) -> u8 {
        let base_score = (self.average_device_count as u32 * 100 / MAX_DEVICES_FOR_FULL_SCORE.max(1))
            .min(100) as u8;
        ((base_score as u32 * self.consistency_score as u32) / 100) as u8
    }
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Default,
)]
pub struct PositionMetrics {
    pub declared_position: Position,
    pub position_variance: u32,
    pub triangulation_confirmations: u32,
    pub last_confirmation_block: u64,
}

impl PositionMetrics {
    pub fn record_triangulation(&mut self, confirmed_position: Position, block: u64) {
        let dist_sq = self.declared_position.distance_squared(&confirmed_position);
        let dist_cm = integer_sqrt(dist_sq) as u32;

        let prev_var = self.position_variance as u64;
        let confirmations = self.triangulation_confirmations.saturating_add(1);
        let new_var = (prev_var.saturating_mul(self.triangulation_confirmations as u64)
            .saturating_add(dist_cm as u64)) / confirmations as u64;

        self.position_variance = new_var.min(u32::MAX as u64) as u32;
        self.triangulation_confirmations = confirmations;
        self.last_confirmation_block = block;
    }

    pub fn position_score(&self) -> u8 {
        if self.triangulation_confirmations < MIN_TRIANGULATION_NODES {
            return 50;
        }

        let max_variance = POSITION_TOLERANCE_CM;
        if self.position_variance >= max_variance {
            return 0;
        }

        let score = 100u32.saturating_sub(
            (self.position_variance.saturating_mul(100)) / max_variance
        );
        score.min(100) as u8
    }
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Default,
)]
pub struct FusedHealthMetrics {
    pub heartbeat_score: u8,
    pub device_metrics: DeviceObservationMetrics,
    pub position_metrics: PositionMetrics,
    pub fused_score: u8,
    pub last_update_block: u64,
}

impl FusedHealthMetrics {
    pub fn new(position: Position) -> Self {
        Self {
            heartbeat_score: 100,
            device_metrics: DeviceObservationMetrics::default(),
            position_metrics: PositionMetrics {
                declared_position: position,
                ..Default::default()
            },
            fused_score: 100,
            last_update_block: 0,
        }
    }

    pub fn recalculate_fused_score(&mut self, weights: &FusionWeights) {
        let heartbeat_component = (self.heartbeat_score as u32)
            .saturating_mul(weights.heartbeat_weight as u32);

        let device_component = (self.device_metrics.device_score() as u32)
            .saturating_mul(weights.device_weight as u32);

        let position_component = (self.position_metrics.position_score() as u32)
            .saturating_mul(weights.position_weight as u32);

        let total = heartbeat_component
            .saturating_add(device_component)
            .saturating_add(position_component);

        self.fused_score = (total / 100).min(100) as u8;
    }

    pub fn update_heartbeat(&mut self, score: u8, block: u64, weights: &FusionWeights) {
        self.heartbeat_score = score;
        self.last_update_block = block;
        self.recalculate_fused_score(weights);
    }

    pub fn record_device_observation(
        &mut self,
        device_count: u8,
        block: u64,
        commitment: H256,
        weights: &FusionWeights,
    ) {
        self.device_metrics.record_observation(device_count, block, commitment);
        self.last_update_block = block;
        self.recalculate_fused_score(weights);
    }

    pub fn record_position_confirmation(
        &mut self,
        position: Position,
        block: u64,
        weights: &FusionWeights,
    ) {
        self.position_metrics.record_triangulation(position, block);
        self.last_update_block = block;
        self.recalculate_fused_score(weights);
    }

    pub fn is_critical(&self) -> bool {
        self.fused_score < CRITICAL_HEALTH_THRESHOLD
    }

    pub fn is_warning(&self) -> bool {
        self.fused_score < WARNING_HEALTH_THRESHOLD && self.fused_score >= CRITICAL_HEALTH_THRESHOLD
    }

    pub fn is_healthy(&self) -> bool {
        self.fused_score >= WARNING_HEALTH_THRESHOLD
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum HealingTrigger {
    HeartbeatTimeout,
    DeviceObservationMissing,
    PositionMismatch,
    FusedScoreCritical,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct HealingAction {
    pub trigger: HealingTrigger,
    pub block: u64,
    pub previous_score: u8,
}

pub fn compute_fused_health(
    heartbeat_score: u8,
    device_count: u8,
    consistency: u8,
    position_variance: u32,
    triangulation_confirmations: u32,
) -> u8 {
    let device_base = (device_count as u32 * 100 / MAX_DEVICES_FOR_FULL_SCORE.max(1)).min(100) as u8;
    let device_score = ((device_base as u32 * consistency as u32) / 100) as u8;

    let position_score = if triangulation_confirmations < MIN_TRIANGULATION_NODES {
        50u8
    } else if position_variance >= POSITION_TOLERANCE_CM {
        0u8
    } else {
        (100u32.saturating_sub((position_variance * 100) / POSITION_TOLERANCE_CM)).min(100) as u8
    };

    let total = (heartbeat_score as u32 * ALPHA as u32)
        .saturating_add(device_score as u32 * BETA as u32)
        .saturating_add(position_score as u32 * GAMMA as u32);

    (total / 100).min(100) as u8
}

pub fn should_trigger_healing(metrics: &FusedHealthMetrics, current_block: u64) -> Option<HealingTrigger> {
    if metrics.fused_score < CRITICAL_HEALTH_THRESHOLD {
        return Some(HealingTrigger::FusedScoreCritical);
    }

    if metrics.heartbeat_score < 30 {
        return Some(HealingTrigger::HeartbeatTimeout);
    }

    if metrics.device_metrics.total_observations > 0 {
        let blocks_since_reveal = current_block.saturating_sub(metrics.device_metrics.last_reveal_block);
        if blocks_since_reveal > 100 {
            return Some(HealingTrigger::DeviceObservationMissing);
        }
    }

    if metrics.position_metrics.triangulation_confirmations >= MIN_TRIANGULATION_NODES {
        if metrics.position_metrics.position_variance > POSITION_TOLERANCE_CM {
            return Some(HealingTrigger::PositionMismatch);
        }
    }

    None
}

fn integer_sqrt(n: u128) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x.min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fusion_weights_validation() {
        assert!(FusionWeights::new(40, 40, 20).is_some());
        assert!(FusionWeights::new(50, 50, 10).is_none());
        assert!(FusionWeights::new(0, 0, 100).is_some());
    }

    #[test]
    fn test_position_distance() {
        let p1 = Position::new(0, 0, 0);
        let p2 = Position::new(100, 0, 0);
        assert_eq!(p1.distance_squared(&p2), 10000);

        assert!(p1.within_tolerance(&p2, 100));
        assert!(!p1.within_tolerance(&p2, 99));
    }

    #[test]
    fn test_device_score_calculation() {
        let mut metrics = DeviceObservationMetrics::default();
        metrics.consistency_score = 100;
        metrics.average_device_count = 10;
        assert_eq!(metrics.device_score(), 100);

        metrics.average_device_count = 5;
        assert_eq!(metrics.device_score(), 50);
    }

    #[test]
    fn test_fused_health_calculation() {
        let score = compute_fused_health(100, 10, 100, 0, 3);
        assert_eq!(score, 100);

        let score = compute_fused_health(0, 0, 0, POSITION_TOLERANCE_CM, 3);
        assert_eq!(score, 0);

        let score = compute_fused_health(50, 5, 50, 2500, 3);
        assert!(score > 0 && score < 100);
    }

    #[test]
    fn test_healing_triggers() {
        let mut metrics = FusedHealthMetrics::new(Position::default());
        metrics.fused_score = 15;
        assert_eq!(
            should_trigger_healing(&metrics, 100),
            Some(HealingTrigger::FusedScoreCritical)
        );

        metrics.fused_score = 50;
        metrics.heartbeat_score = 20;
        assert_eq!(
            should_trigger_healing(&metrics, 100),
            Some(HealingTrigger::HeartbeatTimeout)
        );
    }
}
