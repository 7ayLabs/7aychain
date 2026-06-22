use std::path::PathBuf;
use std::time::Duration;

use rand::SeedableRng;
use rand::{Rng, RngCore};
use rand_chacha::ChaCha8Rng;
use sp_core::H256;

use super::state::{write_status_file, CarrierSignalHandle};
use super::types::CarrierSignalSample;

pub struct MockSourceConfig {
    pub carrier_id: String,
    pub region_id: H256,
    pub network_type: String,
    pub status_file: Option<PathBuf>,
    pub interval: Duration,
    pub seed: u64,
}

/// Synthetic carrier-signal generator. Models a slow random walk around
/// a stable RSSI baseline plus occasional shocks (handover, fade).
/// Useful for devnets where no real radio is available.
pub async fn run_mock_source(config: MockSourceConfig, handle: CarrierSignalHandle) {
    log::info!(
        "carrier-signal: mock source started · carrier={} region={:?} net={} interval={}s seed={}",
        config.carrier_id,
        config.region_id,
        config.network_type,
        config.interval.as_secs(),
        config.seed
    );

    let mut rng = ChaCha8Rng::seed_from_u64(config.seed);
    let mut rssi = -72_i16;
    let mut sequence: u64 = 0;

    loop {
        // Random walk on RSSI bounded to [-110, -45].
        let delta = rng.gen_range(-3..=3);
        rssi = (rssi + delta).clamp(-110, -45);
        // 5% chance of a fade event subtracting 10 dB transiently.
        if rng.next_u32() % 20 == 0 {
            rssi = (rssi - 10).max(-115);
        }

        let bars = rssi_to_bars(rssi);
        let snr = rssi_to_snr(rssi);
        let latency_ms: u32 = rng.gen_range(20..=80);
        let jitter_ms: u32 = rng.gen_range(0..=8);
        let packet_loss_pct: u8 = if rng.next_u32() % 10 == 0 {
            rng.gen_range(1..=5)
        } else {
            0
        };

        sequence = sequence.saturating_add(1);
        let sample = CarrierSignalSample {
            carrier_id: config.carrier_id.clone(),
            region_id: config.region_id,
            rssi_dbm: rssi,
            snr_db: snr,
            bars,
            network_type: config.network_type.clone(),
            latency_ms,
            jitter_ms,
            packet_loss_pct,
            captured_at: current_unix_time(),
            sequence,
        };

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

        tokio::time::sleep(config.interval).await;
    }
}

fn rssi_to_bars(rssi: i16) -> u8 {
    match rssi {
        x if x > -55 => 5,
        x if x > -70 => 4,
        x if x > -85 => 3,
        x if x > -100 => 2,
        x if x > -115 => 1,
        _ => 0,
    }
}

fn rssi_to_snr(rssi: i16) -> i16 {
    match rssi {
        x if x > -60 => 30,
        x if x > -75 => 22,
        x if x > -90 => 14,
        x if x > -105 => 6,
        _ => 0,
    }
}

fn current_unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bars_monotonic_with_rssi() {
        assert_eq!(rssi_to_bars(-40), 5);
        assert_eq!(rssi_to_bars(-65), 4);
        assert_eq!(rssi_to_bars(-80), 3);
        assert_eq!(rssi_to_bars(-95), 2);
        assert_eq!(rssi_to_bars(-110), 1);
        assert_eq!(rssi_to_bars(-130), 0);
    }
}
