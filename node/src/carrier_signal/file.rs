use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use sp_core::H256;

use super::state::{write_status_file, CarrierSignalHandle};
use super::types::CarrierSignalSample;

/// Wire format the node accepts on disk. A separate tool (devnet bridge,
/// modem reader, etc.) writes this; the node polls and forwards.
#[derive(Debug, Deserialize)]
struct CarrierSignalFile {
    carrier_id: String,
    region_id: String,
    rssi_dbm: i16,
    #[serde(default)]
    snr_db: Option<i16>,
    #[serde(default)]
    bars: Option<u8>,
    network_type: String,
    #[serde(default)]
    latency_ms: Option<u32>,
    #[serde(default)]
    jitter_ms: Option<u32>,
    #[serde(default)]
    packet_loss_pct: Option<u8>,
    #[serde(default)]
    captured_at: Option<u64>,
}

pub struct FileSourceConfig {
    pub input_file: PathBuf,
    pub status_file: Option<PathBuf>,
    pub interval: Duration,
}

pub async fn run_file_source(config: FileSourceConfig, handle: CarrierSignalHandle) {
    log::info!(
        "carrier-signal: file source started · input={} interval={}s",
        config.input_file.display(),
        config.interval.as_secs()
    );

    let mut sequence: u64 = 0;

    loop {
        match std::fs::read(&config.input_file) {
            Ok(bytes) => match parse_file(&bytes) {
                Ok(parsed) => {
                    sequence = sequence.saturating_add(1);
                    let sample = parsed.into_sample(sequence);
                    log_sample(&sample);
                    handle.write(sample.clone());
                    if let Some(status) = config.status_file.as_deref() {
                        if let Err(error) = write_status_file(status, &sample) {
                            log::warn!(
                                "carrier-signal: status write failed ({}): {}",
                                status.display(),
                                error
                            );
                        }
                    }
                }
                Err(error) => {
                    log::warn!(
                        "carrier-signal: rejected {} ({})",
                        config.input_file.display(),
                        error
                    );
                }
            },
            Err(error) => {
                log::debug!(
                    "carrier-signal: cannot read {} ({})",
                    config.input_file.display(),
                    error
                );
            }
        }

        tokio::time::sleep(config.interval).await;
    }
}

fn log_sample(sample: &CarrierSignalSample) {
    log::info!(
        "carrier-signal: seq={} carrier={} net={} rssi={}dBm bars={}/5 lat={}ms loss={}%",
        sample.sequence,
        sample.carrier_id,
        sample.network_type,
        sample.rssi_dbm,
        sample.bars,
        sample.latency_ms,
        sample.packet_loss_pct,
    );
}

struct ParsedSample {
    carrier_id: String,
    region_id: H256,
    rssi_dbm: i16,
    snr_db: i16,
    bars: u8,
    network_type: String,
    latency_ms: u32,
    jitter_ms: u32,
    packet_loss_pct: u8,
    captured_at: u64,
}

impl ParsedSample {
    fn into_sample(self, sequence: u64) -> CarrierSignalSample {
        CarrierSignalSample {
            carrier_id: self.carrier_id,
            region_id: self.region_id,
            rssi_dbm: self.rssi_dbm,
            snr_db: self.snr_db,
            bars: self.bars,
            network_type: self.network_type,
            latency_ms: self.latency_ms,
            jitter_ms: self.jitter_ms,
            packet_loss_pct: self.packet_loss_pct,
            captured_at: self.captured_at,
            sequence,
        }
    }
}

fn parse_file(bytes: &[u8]) -> Result<ParsedSample, String> {
    let raw: CarrierSignalFile =
        serde_json::from_slice(bytes).map_err(|e| format!("invalid json: {e}"))?;

    if raw.carrier_id.is_empty() || raw.carrier_id.len() > super::types::MAX_CARRIER_ID_LEN {
        return Err(format!(
            "carrier_id length {} out of bounds (1..={})",
            raw.carrier_id.len(),
            super::types::MAX_CARRIER_ID_LEN
        ));
    }
    if raw.network_type.is_empty() || raw.network_type.len() > super::types::MAX_NETWORK_TYPE_LEN {
        return Err(format!(
            "network_type length {} out of bounds (1..={})",
            raw.network_type.len(),
            super::types::MAX_NETWORK_TYPE_LEN
        ));
    }
    if !(-150..=0).contains(&raw.rssi_dbm) {
        return Err(format!("rssi_dbm {} outside [-150,0]", raw.rssi_dbm));
    }
    let region_id = parse_h256(&raw.region_id)?;
    let snr_db = raw.snr_db.unwrap_or(infer_snr(raw.rssi_dbm));
    if !(-10..=60).contains(&snr_db) {
        return Err(format!("snr_db {} outside [-10,60]", snr_db));
    }
    let bars = raw.bars.unwrap_or_else(|| infer_bars(raw.rssi_dbm));
    if bars > 5 {
        return Err(format!("bars {} out of range 0..=5", bars));
    }
    let packet_loss_pct = raw.packet_loss_pct.unwrap_or(0);
    if packet_loss_pct > 100 {
        return Err(format!("packet_loss_pct {} > 100", packet_loss_pct));
    }
    Ok(ParsedSample {
        carrier_id: raw.carrier_id,
        region_id,
        rssi_dbm: raw.rssi_dbm,
        snr_db,
        bars,
        network_type: raw.network_type,
        latency_ms: raw.latency_ms.unwrap_or(0),
        jitter_ms: raw.jitter_ms.unwrap_or(0),
        packet_loss_pct,
        captured_at: raw.captured_at.unwrap_or_else(current_unix_time),
    })
}

fn parse_h256(input: &str) -> Result<H256, String> {
    let trimmed = input.strip_prefix("0x").unwrap_or(input);
    if trimmed.len() != 64 {
        return Err(format!(
            "region_id must be 32 bytes hex (got {} chars)",
            trimmed.len()
        ));
    }
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(trimmed, &mut bytes).map_err(|e| format!("invalid hex: {e}"))?;
    Ok(H256(bytes))
}

fn infer_snr(rssi_dbm: i16) -> i16 {
    // Rough heuristic so callers don't have to provide both.
    match rssi_dbm {
        x if x > -60 => 30,
        x if x > -75 => 22,
        x if x > -90 => 14,
        x if x > -105 => 6,
        _ => 0,
    }
}

fn infer_bars(rssi_dbm: i16) -> u8 {
    match rssi_dbm {
        x if x > -55 => 5,
        x if x > -70 => 4,
        x if x > -85 => 3,
        x if x > -100 => 2,
        x if x > -115 => 1,
        _ => 0,
    }
}

fn current_unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[allow(dead_code)]
pub(crate) fn parse_for_test(bytes: &[u8]) -> Result<CarrierSignalSample, String> {
    parse_file(bytes).map(|p| p.into_sample(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_payload() {
        let json = br#"{
            "carrier_id": "7AY-Test",
            "region_id": "0x1111111111111111111111111111111111111111111111111111111111111111",
            "rssi_dbm": -67,
            "snr_db": 18,
            "bars": 4,
            "network_type": "LTE",
            "latency_ms": 42,
            "jitter_ms": 3,
            "packet_loss_pct": 1,
            "captured_at": 1700000000
        }"#;
        let s = parse_for_test(json).expect("parse");
        assert_eq!(s.rssi_dbm, -67);
        assert_eq!(s.bars, 4);
        assert_eq!(s.region_id, H256::repeat_byte(0x11));
        assert_eq!(s.sequence, 1);
    }

    #[test]
    fn infers_optional_fields() {
        // -78 dBm sits in the "bars=3, snr=14" bucket per the heuristic.
        let json = br#"{
            "carrier_id": "7AY-Test",
            "region_id": "0x1111111111111111111111111111111111111111111111111111111111111111",
            "rssi_dbm": -78,
            "network_type": "LTE"
        }"#;
        let s = parse_for_test(json).expect("parse");
        assert_eq!(s.snr_db, 14);
        assert_eq!(s.bars, 3);
        assert_eq!(s.latency_ms, 0);
    }

    #[test]
    fn rejects_short_region_id() {
        let json = br#"{
            "carrier_id": "X",
            "region_id": "0x1234",
            "rssi_dbm": -80,
            "network_type": "LTE"
        }"#;
        let e = parse_for_test(json).expect_err("should fail");
        assert!(e.contains("region_id"));
    }

    #[test]
    fn rejects_out_of_range_rssi() {
        let json = br#"{
            "carrier_id": "X",
            "region_id": "0x1111111111111111111111111111111111111111111111111111111111111111",
            "rssi_dbm": 5,
            "network_type": "LTE"
        }"#;
        let e = parse_for_test(json).expect_err("should fail");
        assert!(e.contains("rssi_dbm"));
    }

    #[test]
    fn rejects_long_carrier_id() {
        let long_id = "A".repeat(40);
        let json = format!(
            r#"{{
                "carrier_id": "{long_id}",
                "region_id": "0x1111111111111111111111111111111111111111111111111111111111111111",
                "rssi_dbm": -80,
                "network_type": "LTE"
            }}"#
        );
        let e = parse_for_test(json.as_bytes()).expect_err("should fail");
        assert!(e.contains("carrier_id"));
    }
}
