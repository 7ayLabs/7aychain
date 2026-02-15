//! Network latency-based presence scanner.
//!
//! Measures round-trip time to peers for distance estimation
//! without requiring WiFi/Bluetooth hardware.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use sc_network::PeerId;

/// Speed of network propagation (~150 km per millisecond).
pub const NETWORK_SPEED_KM_PER_MS: u32 = 150;

/// Maximum valid latency (prevents gaming with artificial delays).
pub const MAX_VALID_LATENCY_MS: u32 = 1000;

/// Latency measurement result for a peer.
#[derive(Clone, Debug)]
pub struct PeerLatency {
    /// Peer identifier
    pub peer_id: PeerId,
    /// Round-trip time in milliseconds
    pub rtt_ms: u32,
    /// Whether this is a direct connection
    pub direct: bool,
    /// Estimated number of hops
    pub hops: Option<u8>,
    /// When the measurement was taken
    pub measured_at: Instant,
    /// Maximum possible distance in km
    pub max_distance_km: u32,
}

impl PeerLatency {
    /// Create from a raw RTT measurement.
    pub fn new(peer_id: PeerId, rtt_ms: u32, direct: bool) -> Self {
        let max_distance_km = (rtt_ms / 2) * NETWORK_SPEED_KM_PER_MS;
        Self {
            peer_id,
            rtt_ms,
            direct,
            hops: None,
            measured_at: Instant::now(),
            max_distance_km,
        }
    }

    /// Check if the measurement is valid.
    pub fn is_valid(&self) -> bool {
        self.rtt_ms > 0 && self.rtt_ms <= MAX_VALID_LATENCY_MS
    }
}

/// Network latency scanner configuration.
#[derive(Clone, Debug)]
pub struct LatencyScannerConfig {
    /// How often to measure peer latency
    pub scan_interval: Duration,
    /// Timeout for ping requests
    pub ping_timeout: Duration,
    /// Maximum peers to measure per scan
    pub max_peers_per_scan: usize,
}

impl Default for LatencyScannerConfig {
    fn default() -> Self {
        Self {
            scan_interval: Duration::from_secs(10),
            ping_timeout: Duration::from_secs(5),
            max_peers_per_scan: 50,
        }
    }
}

/// Latency scanner that measures RTT to network peers.
pub struct LatencyScanner {
    config: LatencyScannerConfig,
    /// Cache of recent measurements
    measurements: HashMap<PeerId, PeerLatency>,
}

impl LatencyScanner {
    /// Create a new latency scanner.
    pub fn new(config: LatencyScannerConfig) -> Self {
        Self {
            config,
            measurements: HashMap::new(),
        }
    }

    /// Update with new peer latency data from the network layer.
    /// This is called when the network reports peer statistics.
    pub fn update_peer_latency(&mut self, peer_id: PeerId, rtt_ms: u32, direct: bool) {
        let measurement = PeerLatency::new(peer_id, rtt_ms, direct);
        if measurement.is_valid() {
            self.measurements.insert(peer_id, measurement);
        }
    }

    /// Get all current latency measurements.
    pub fn get_measurements(&self) -> Vec<PeerLatency> {
        self.measurements.values().cloned().collect()
    }

    /// Get latency measurement for a specific peer.
    pub fn get_peer_latency(&self, peer_id: &PeerId) -> Option<&PeerLatency> {
        self.measurements.get(peer_id)
    }

    /// Clear old measurements (older than TTL).
    pub fn cleanup_old_measurements(&mut self, ttl: Duration) {
        let cutoff = Instant::now() - ttl;
        self.measurements.retain(|_, m| m.measured_at > cutoff);
    }

    /// Get statistics about current measurements.
    pub fn get_statistics(&self) -> LatencyStatistics {
        let valid_measurements: Vec<_> = self.measurements.values().filter(|m| m.is_valid()).collect();

        if valid_measurements.is_empty() {
            return LatencyStatistics::default();
        }

        let total_rtt: u32 = valid_measurements.iter().map(|m| m.rtt_ms).sum();
        let avg_rtt = total_rtt / valid_measurements.len() as u32;

        let direct_count = valid_measurements.iter().filter(|m| m.direct).count();

        let min_rtt = valid_measurements.iter().map(|m| m.rtt_ms).min().unwrap_or(0);
        let max_rtt = valid_measurements.iter().map(|m| m.rtt_ms).max().unwrap_or(0);

        LatencyStatistics {
            peer_count: valid_measurements.len() as u32,
            avg_rtt_ms: avg_rtt,
            min_rtt_ms: min_rtt,
            max_rtt_ms: max_rtt,
            direct_connections: direct_count as u32,
            relayed_connections: (valid_measurements.len() - direct_count) as u32,
        }
    }
}

/// Statistics about latency measurements.
#[derive(Clone, Debug, Default)]
pub struct LatencyStatistics {
    /// Number of peers measured
    pub peer_count: u32,
    /// Average RTT in ms
    pub avg_rtt_ms: u32,
    /// Minimum RTT in ms
    pub min_rtt_ms: u32,
    /// Maximum RTT in ms
    pub max_rtt_ms: u32,
    /// Number of direct connections
    pub direct_connections: u32,
    /// Number of relayed connections
    pub relayed_connections: u32,
}

/// Mock latency scanner for testing.
pub struct MockLatencyScanner {
    base_latency_ms: u32,
    variance_ms: u32,
    seed: u64,
}

impl MockLatencyScanner {
    /// Create a new mock latency scanner.
    pub fn new(base_latency_ms: u32, variance_ms: u32, seed: u64) -> Self {
        Self {
            base_latency_ms,
            variance_ms,
            seed,
        }
    }

    /// Generate mock latency measurements.
    pub fn generate_measurements(&mut self, peer_count: usize) -> Vec<PeerLatency> {
        let mut measurements = Vec::with_capacity(peer_count);

        for i in 0..peer_count {
            // Simple deterministic "random" based on seed
            let variance = ((self.seed.wrapping_mul(i as u64 + 1)) % (self.variance_ms as u64 * 2)) as u32;
            let rtt = self.base_latency_ms + variance - self.variance_ms;

            // Create a mock peer ID
            let mut peer_bytes = [0u8; 38];
            peer_bytes[0..8].copy_from_slice(&(i as u64).to_le_bytes());
            let peer_id = PeerId::from_bytes(&peer_bytes).unwrap_or_else(|_| PeerId::random());

            let direct = i % 3 != 0; // 2/3 direct connections

            measurements.push(PeerLatency::new(peer_id, rtt, direct));
        }

        self.seed = self.seed.wrapping_add(1);
        measurements
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_latency_distance() {
        let latency = PeerLatency::new(PeerId::random(), 10, true);
        // 10ms RTT -> 5ms one-way -> 5 * 150 = 750km
        assert_eq!(latency.max_distance_km, 750);
    }

    #[test]
    fn test_latency_validation() {
        let valid = PeerLatency::new(PeerId::random(), 100, true);
        assert!(valid.is_valid());

        let too_high = PeerLatency::new(PeerId::random(), 2000, true);
        assert!(!too_high.is_valid());
    }

    #[test]
    fn test_scanner_statistics() {
        let mut scanner = LatencyScanner::new(LatencyScannerConfig::default());

        scanner.update_peer_latency(PeerId::random(), 10, true);
        scanner.update_peer_latency(PeerId::random(), 20, true);
        scanner.update_peer_latency(PeerId::random(), 30, false);

        let stats = scanner.get_statistics();
        assert_eq!(stats.peer_count, 3);
        assert_eq!(stats.avg_rtt_ms, 20);
        assert_eq!(stats.min_rtt_ms, 10);
        assert_eq!(stats.max_rtt_ms, 30);
        assert_eq!(stats.direct_connections, 2);
        assert_eq!(stats.relayed_connections, 1);
    }

    #[test]
    fn test_mock_scanner() {
        let mut mock = MockLatencyScanner::new(50, 20, 42);
        let measurements = mock.generate_measurements(5);

        assert_eq!(measurements.len(), 5);
        for m in &measurements {
            assert!(m.is_valid());
        }
    }
}
