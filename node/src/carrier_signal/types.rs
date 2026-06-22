use serde::{Deserialize, Serialize};
use sp_core::H256;

pub const MAX_CARRIER_ID_LEN: usize = 32;
pub const MAX_NETWORK_TYPE_LEN: usize = 16;

/// Snapshot of the local carrier signal as observed by this node.
///
/// "Each node, each person on its carrier" — one sample represents what
/// the running node currently sees from its upstream carrier (real or
/// mock). The chain consumes this via a Python bridge that submits
/// `pallet-carrier::refresh_signal_quality` and related extrinsics.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarrierSignalSample {
    /// Free-form carrier label, e.g. `"7AY-Test"`, `"VZW"`, `"COMM-MESH-04"`.
    pub carrier_id: String,
    /// Region this sample applies to (matches `pallet-carrier` region commitment).
    pub region_id: H256,
    /// Received Signal Strength Indicator in dBm. Typical range: -120 (weak) .. -30 (strong).
    pub rssi_dbm: i16,
    /// Signal-to-noise ratio in dB. Typical range: 0 .. 40.
    pub snr_db: i16,
    /// Coverage bars, 0..5 (UI-friendly normalization of rssi/snr).
    pub bars: u8,
    /// Network technology, e.g. `"LTE"`, `"5G"`, `"7AY-MESH"`, `"WCDMA"`.
    pub network_type: String,
    /// Round-trip latency to the upstream serving node in ms.
    pub latency_ms: u32,
    /// Jitter in ms over the last sampling window.
    pub jitter_ms: u32,
    /// Packet loss in percent over the last sampling window.
    pub packet_loss_pct: u8,
    /// Unix epoch seconds when the sample was captured by the source.
    pub captured_at: u64,
    /// Monotonic sequence number assigned by this node.
    pub sequence: u64,
}

impl CarrierSignalSample {
    pub fn is_valid(&self) -> bool {
        self.carrier_id.len() <= MAX_CARRIER_ID_LEN
            && !self.carrier_id.is_empty()
            && self.network_type.len() <= MAX_NETWORK_TYPE_LEN
            && (-150..=0).contains(&self.rssi_dbm)
            && (-10..=60).contains(&self.snr_db)
            && self.bars <= 5
            && self.packet_loss_pct <= 100
    }
}
