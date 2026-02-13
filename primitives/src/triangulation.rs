#![allow(clippy::result_unit_err)]

use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::Perbill;

use crate::fusion::Position;

pub const TX_POWER_DEFAULT: i8 = -59;
pub const PATH_LOSS_EXPONENT_FREE_SPACE: f64 = 2.0;
pub const PATH_LOSS_EXPONENT_INDOOR: f64 = 2.7;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct TriangulationConfig {
    pub tx_power: i8,
    pub path_loss_exponent_x100: u16,
    pub min_signals: u8,
    pub max_distance_meters: u32,
    pub confidence_threshold: u8,
}

impl Default for TriangulationConfig {
    fn default() -> Self {
        Self {
            tx_power: TX_POWER_DEFAULT,
            path_loss_exponent_x100: 270,
            min_signals: 3,
            max_distance_meters: 100,
            confidence_threshold: 50,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct SignalObservation {
    pub observer_position: Position,
    pub rssi: i8,
    pub frequency_mhz: Option<u16>,
    pub timestamp: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct TriangulatedPosition {
    pub position: Position,
    pub confidence: Perbill,
    pub signal_count: u8,
    pub average_distance: u32,
    pub variance: u32,
}

pub fn rssi_to_distance_cm(rssi: i8, tx_power: i8, path_loss_x100: u16) -> u32 {
    let rssi_diff = tx_power.saturating_sub(rssi) as i32;
    if rssi_diff <= 0 {
        return 10;
    }

    let path_loss = path_loss_x100 as i32;
    let exponent_scaled = (rssi_diff * 100) / (path_loss.max(1) * 10);

    let mut distance_cm: u32 = 100;
    for _ in 0..exponent_scaled.min(20) {
        distance_cm = distance_cm.saturating_mul(10);
    }

    let remainder = (exponent_scaled % 10) as u32;
    distance_cm = distance_cm.saturating_add(distance_cm * remainder / 10);

    distance_cm.max(10).min(100_000_00)
}

pub fn calculate_weighted_centroid(observations: &[SignalObservation], config: &TriangulationConfig) -> Option<TriangulatedPosition> {
    if observations.len() < config.min_signals as usize {
        return None;
    }

    let weights: Vec<u64> = observations
        .iter()
        .map(|obs| {
            let dist = rssi_to_distance_cm(obs.rssi, config.tx_power, config.path_loss_exponent_x100);
            if dist == 0 {
                1
            } else {
                (1_000_000_u64) / (dist as u64 * dist as u64).max(1)
            }
        })
        .collect();

    let total_weight: u64 = weights.iter().sum();
    if total_weight == 0 {
        return None;
    }

    let weighted_x: i64 = observations
        .iter()
        .zip(weights.iter())
        .map(|(obs, &w)| (obs.observer_position.x as i64) * (w as i64))
        .sum();

    let weighted_y: i64 = observations
        .iter()
        .zip(weights.iter())
        .map(|(obs, &w)| (obs.observer_position.y as i64) * (w as i64))
        .sum();

    let weighted_z: i64 = observations
        .iter()
        .zip(weights.iter())
        .map(|(obs, &w)| (obs.observer_position.z as i64) * (w as i64))
        .sum();

    let position = Position {
        x: (weighted_x / total_weight as i64) as i32,
        y: (weighted_y / total_weight as i64) as i32,
        z: (weighted_z / total_weight as i64) as i32,
    };

    let distances: Vec<u32> = observations
        .iter()
        .map(|obs| rssi_to_distance_cm(obs.rssi, config.tx_power, config.path_loss_exponent_x100))
        .collect();

    let avg_distance = distances.iter().sum::<u32>() / distances.len().max(1) as u32;

    let variance = if distances.len() > 1 {
        let mean = avg_distance as i64;
        let sum_sq: i64 = distances
            .iter()
            .map(|&d| {
                let diff = d as i64 - mean;
                diff * diff
            })
            .sum();
        (sum_sq / distances.len() as i64) as u32
    } else {
        0
    };

    let signal_factor = (observations.len() as u32 * 20).min(60);
    let variance_penalty = (variance / 100).min(40);
    let confidence_percent = signal_factor.saturating_sub(variance_penalty).min(100);
    let confidence = Perbill::from_percent(confidence_percent);

    Some(TriangulatedPosition {
        position,
        confidence,
        signal_count: observations.len() as u8,
        average_distance: avg_distance,
        variance,
    })
}

pub fn multilateration(observations: &[SignalObservation], config: &TriangulationConfig) -> Option<TriangulatedPosition> {
    if observations.len() < 3 {
        return calculate_weighted_centroid(observations, config);
    }

    let distances: Vec<(Position, u32)> = observations
        .iter()
        .map(|obs| {
            let dist = rssi_to_distance_cm(obs.rssi, config.tx_power, config.path_loss_exponent_x100);
            (obs.observer_position, dist)
        })
        .collect();

    let (p1, d1) = distances[0];
    let (p2, d2) = distances[1];
    let (p3, d3) = distances[2];

    let x1 = p1.x as i64;
    let y1 = p1.y as i64;
    let x2 = p2.x as i64;
    let y2 = p2.y as i64;
    let x3 = p3.x as i64;
    let y3 = p3.y as i64;

    let r1 = (d1 / 100) as i64;
    let r2 = (d2 / 100) as i64;
    let r3 = (d3 / 100) as i64;

    let a = 2 * (x2 - x1);
    let b = 2 * (y2 - y1);
    let c = r1 * r1 - r2 * r2 - x1 * x1 + x2 * x2 - y1 * y1 + y2 * y2;
    let d = 2 * (x3 - x2);
    let e = 2 * (y3 - y2);
    let f = r2 * r2 - r3 * r3 - x2 * x2 + x3 * x3 - y2 * y2 + y3 * y3;

    let denom = a * e - b * d;
    if denom == 0 {
        return calculate_weighted_centroid(observations, config);
    }

    let x = (c * e - f * b) / denom;
    let y = (a * f - c * d) / denom;

    let position = Position::new(x as i32, y as i32, p1.z);

    let avg_distance = (d1 + d2 + d3) / 3;
    let confidence = Perbill::from_percent(70);

    Some(TriangulatedPosition {
        position,
        confidence,
        signal_count: observations.len() as u8,
        average_distance: avg_distance,
        variance: 0,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct DeviceTrack {
    pub device_hash: H256,
    pub positions: Vec<TriangulatedPosition>,
    pub last_seen: u64,
    pub velocity: Option<Velocity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct Velocity {
    pub dx: i32,
    pub dy: i32,
    pub dz: i32,
    pub speed_cm_per_sec: u32,
}

impl DeviceTrack {
    pub fn new(device_hash: H256, initial_position: TriangulatedPosition, timestamp: u64) -> Self {
        Self {
            device_hash,
            positions: alloc::vec![initial_position],
            last_seen: timestamp,
            velocity: None,
        }
    }

    pub fn update(&mut self, new_position: TriangulatedPosition, timestamp: u64) {
        if let Some(last) = self.positions.last() {
            let time_diff = timestamp.saturating_sub(self.last_seen);
            if time_diff > 0 {
                let dx = new_position.position.x - last.position.x;
                let dy = new_position.position.y - last.position.y;
                let dz = new_position.position.z - last.position.z;

                let distance_squared = (dx as i64 * dx as i64 + dy as i64 * dy as i64 + dz as i64 * dz as i64) as u64;
                let distance = integer_sqrt(distance_squared);
                let speed = ((distance * 100) / time_diff) as u32;

                self.velocity = Some(Velocity {
                    dx,
                    dy,
                    dz,
                    speed_cm_per_sec: speed,
                });
            }
        }

        self.positions.push(new_position);
        self.last_seen = timestamp;

        if self.positions.len() > 10 {
            self.positions.remove(0);
        }
    }

    pub fn predict_position(&self, future_seconds: u32) -> Option<Position> {
        let current = self.positions.last()?;
        let velocity = self.velocity?;

        Some(Position {
            x: current.position.x + (velocity.dx * future_seconds as i32) / 100,
            y: current.position.y + (velocity.dy * future_seconds as i32) / 100,
            z: current.position.z + (velocity.dz * future_seconds as i32) / 100,
        })
    }
}

fn integer_sqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rssi_to_distance() {
        let config = TriangulationConfig::default();

        let strong = rssi_to_distance_cm(-40, config.tx_power, config.path_loss_exponent_x100);
        let weak = rssi_to_distance_cm(-80, config.tx_power, config.path_loss_exponent_x100);

        assert!(weak > strong);
    }

    #[test]
    fn test_weighted_centroid() {
        let config = TriangulationConfig::default();

        let observations = vec![
            SignalObservation {
                observer_position: Position::new(0, 0, 0),
                rssi: -50,
                frequency_mhz: Some(2412),
                timestamp: 1000,
            },
            SignalObservation {
                observer_position: Position::new(100, 0, 0),
                rssi: -50,
                frequency_mhz: Some(2412),
                timestamp: 1000,
            },
            SignalObservation {
                observer_position: Position::new(50, 100, 0),
                rssi: -50,
                frequency_mhz: Some(2412),
                timestamp: 1000,
            },
        ];

        let result = calculate_weighted_centroid(&observations, &config);
        assert!(result.is_some());

        let pos = result.unwrap();
        assert!(pos.position.x >= 40 && pos.position.x <= 60);
    }

    #[test]
    fn test_integer_sqrt() {
        assert_eq!(integer_sqrt(0), 0);
        assert_eq!(integer_sqrt(1), 1);
        assert_eq!(integer_sqrt(4), 2);
        assert_eq!(integer_sqrt(9), 3);
        assert_eq!(integer_sqrt(100), 10);
        assert_eq!(integer_sqrt(10000), 100);
    }

    #[test]
    fn test_device_track_velocity() {
        let initial = TriangulatedPosition {
            position: Position::new(0, 0, 0),
            confidence: Perbill::from_percent(80),
            signal_count: 3,
            average_distance: 500,
            variance: 100,
        };

        let mut track = DeviceTrack::new(H256::zero(), initial, 1000);

        let new_pos = TriangulatedPosition {
            position: Position::new(100, 0, 0),
            confidence: Perbill::from_percent(80),
            signal_count: 3,
            average_distance: 500,
            variance: 100,
        };

        track.update(new_pos, 1010);

        assert!(track.velocity.is_some());
        let vel = track.velocity.unwrap();
        assert_eq!(vel.dx, 100);
        assert_eq!(vel.dy, 0);
    }
}
