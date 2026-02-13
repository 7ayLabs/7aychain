use crate::scanner::types::{DetectedDeviceType, Position, ScanSignalType, ScannedDevice};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sp_core::{blake2_256, H256};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct MockScanner {
    rng: ChaCha8Rng,
    device_pool: Vec<MockDevice>,
    config: MockConfig,
    commitment_history: VecDeque<(H256, [u8; 32], u64)>,
}

#[derive(Clone)]
pub struct MockConfig {
    pub device_count: u32,
    pub device_types: Vec<DetectedDeviceType>,
    pub position: Position,
    pub rssi_range: (i8, i8),
    pub seed: u64,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            device_count: 15,
            device_types: vec![
                DetectedDeviceType::IPhone,
                DetectedDeviceType::Android,
                DetectedDeviceType::MacBook,
                DetectedDeviceType::WindowsPC,
                DetectedDeviceType::IPad,
                DetectedDeviceType::IoTDevice,
            ],
            position: Position::default(),
            rssi_range: (-80, -30),
            seed: 42,
        }
    }
}

struct MockDevice {
    mac_hash: H256,
    device_type: DetectedDeviceType,
    base_rssi: i8,
    visibility_probability: f32,
    movement_offset: (i32, i32),
}

impl MockScanner {
    pub fn new(config: MockConfig) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(config.seed);
        let device_pool = Self::generate_device_pool(&mut rng, &config);

        Self {
            rng,
            device_pool,
            config,
            commitment_history: VecDeque::with_capacity(10),
        }
    }

    fn generate_device_pool(rng: &mut ChaCha8Rng, config: &MockConfig) -> Vec<MockDevice> {
        let count = config.device_count.max(5).min(100);
        (0..count)
            .map(|i| {
                let mut mac_bytes = [0u8; 32];
                rng.fill(&mut mac_bytes);
                let mac_hash = H256(blake2_256(&mac_bytes));

                let device_type = config.device_types[i as usize % config.device_types.len()];
                let base_rssi = rng.gen_range(config.rssi_range.0..config.rssi_range.1);
                let visibility = rng.gen_range(0.3..1.0);
                let movement = (rng.gen_range(-50..50), rng.gen_range(-50..50));

                MockDevice {
                    mac_hash,
                    device_type,
                    base_rssi,
                    visibility_probability: visibility,
                    movement_offset: movement,
                }
            })
            .collect()
    }

    pub async fn scan(&mut self) -> Vec<ScannedDevice> {
        let timestamp = self.get_timestamp();
        let visible_count = self.rng.gen_range(5..(self.config.device_count as usize).min(20));

        let visible_indices: Vec<usize> = self
            .device_pool
            .iter()
            .enumerate()
            .filter(|(_, d)| self.rng.gen::<f32>() < d.visibility_probability)
            .take(visible_count)
            .map(|(i, _)| i)
            .collect();

        let mut devices: Vec<ScannedDevice> = visible_indices
            .into_iter()
            .map(|idx| {
                let mock = &self.device_pool[idx];
                let rssi_variation = self.rng.gen_range(-10i8..10i8);
                let rssi = mock.base_rssi.saturating_add(rssi_variation);

                let signal_roll: f32 = self.rng.gen();
                let signal_type = if signal_roll > 0.3 {
                    ScanSignalType::Wifi
                } else if signal_roll > 0.15 {
                    ScanSignalType::Bluetooth
                } else {
                    ScanSignalType::Ble
                };

                let frequency = match signal_type {
                    ScanSignalType::Wifi => Some(2412 + self.rng.gen_range(0u16..13u16) * 5),
                    ScanSignalType::Bluetooth => None,
                    ScanSignalType::Ble => None,
                };

                ScannedDevice {
                    mac_hash: mock.mac_hash,
                    rssi,
                    signal_type,
                    device_type: mock.device_type,
                    vendor: None,
                    device_name: None,
                    frequency,
                    detected_at: timestamp,
                }
            })
            .collect();

        devices.sort_by(|a, b| b.rssi.cmp(&a.rssi));
        devices
    }

    pub fn generate_commitment(&mut self, block_number: u64) -> (H256, u8) {
        let devices = self.get_current_device_hashes();
        let mut nonce = [0u8; 32];
        self.rng.fill(&mut nonce);

        let merkle_root = self.compute_merkle_root(&devices);
        let commitment = self.compute_commitment(&merkle_root, &nonce, block_number);

        self.commitment_history
            .push_back((commitment, nonce, block_number));
        if self.commitment_history.len() > 10 {
            self.commitment_history.pop_front();
        }

        (commitment, devices.len() as u8)
    }

    pub fn generate_reveal(&mut self, target_block: u64) -> Option<MockReveal> {
        let idx = self
            .commitment_history
            .iter()
            .position(|(_, _, block)| *block == target_block)?;

        let (commitment, nonce, block) = self.commitment_history.remove(idx)?;
        let devices = self.get_current_device_hashes();
        let merkle_root = self.compute_merkle_root(&devices);

        Some(MockReveal {
            commitment_block: block,
            nonce,
            device_merkle_root: merkle_root,
            rssi_values: devices.iter().map(|_| self.rng.gen_range(-80..-30)).collect(),
            original_commitment: commitment,
        })
    }

    fn get_current_device_hashes(&self) -> Vec<H256> {
        self.device_pool.iter().map(|d| d.mac_hash).collect()
    }

    fn compute_merkle_root(&self, hashes: &[H256]) -> H256 {
        if hashes.is_empty() {
            return H256::zero();
        }

        let mut sorted = hashes.to_vec();
        sorted.sort();

        let mut layer = sorted;
        while layer.len() > 1 {
            let mut next = Vec::new();
            for chunk in layer.chunks(2) {
                let left = chunk[0];
                let right = chunk.get(1).copied().unwrap_or(left);
                let combined: Vec<u8> = left
                    .as_bytes()
                    .iter()
                    .chain(right.as_bytes().iter())
                    .copied()
                    .collect();
                next.push(H256(blake2_256(&combined)));
            }
            layer = next;
        }

        layer[0]
    }

    fn compute_commitment(&self, merkle_root: &H256, nonce: &[u8; 32], block: u64) -> H256 {
        let domain = b"7ay:device:commit:v1";
        let mut data = Vec::new();
        data.extend_from_slice(domain);
        data.extend_from_slice(merkle_root.as_bytes());
        data.extend_from_slice(nonce);
        data.extend_from_slice(&block.to_le_bytes());
        H256(blake2_256(&data))
    }

    pub fn get_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn get_position(&self) -> Position {
        self.config.position
    }
}

pub struct MockReveal {
    pub commitment_block: u64,
    pub nonce: [u8; 32],
    pub device_merkle_root: H256,
    pub rssi_values: Vec<i8>,
    pub original_commitment: H256,
}

impl MockReveal {
    pub fn verify(&self, commitment: &H256) -> bool {
        self.original_commitment == *commitment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_scanner_creation() {
        let config = MockConfig::default();
        let scanner = MockScanner::new(config);
        assert!(!scanner.device_pool.is_empty());
    }

    #[tokio::test]
    async fn test_mock_scanner_scan() {
        let config = MockConfig::default();
        let mut scanner = MockScanner::new(config);
        let devices = scanner.scan().await;
        assert!(!devices.is_empty());
        assert!(devices.len() <= 20);
    }

    #[tokio::test]
    async fn test_mock_commitment_reveal() {
        let config = MockConfig::default();
        let mut scanner = MockScanner::new(config);

        let (commitment, count) = scanner.generate_commitment(100);
        assert_ne!(commitment, H256::zero());
        assert!(count > 0);

        let reveal = scanner.generate_reveal(100);
        assert!(reveal.is_some());

        let reveal = reveal.unwrap();
        assert!(reveal.verify(&commitment));
    }

    #[test]
    fn test_merkle_root_deterministic() {
        let config = MockConfig::default();
        let scanner = MockScanner::new(config);

        let hashes = vec![
            H256::repeat_byte(0x01),
            H256::repeat_byte(0x02),
            H256::repeat_byte(0x03),
        ];

        let root1 = scanner.compute_merkle_root(&hashes);
        let root2 = scanner.compute_merkle_root(&hashes);

        assert_eq!(root1, root2);
    }
}
