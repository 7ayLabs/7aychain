use std::sync::{Arc, RwLock};

use super::types::CarrierSignalSample;

/// Shared, lock-protected handle to the latest carrier signal sample.
/// Updated by the sampling task; read by the RPC layer and (optionally)
/// snapshotted to disk.
#[derive(Clone, Default)]
pub struct CarrierSignalHandle {
    inner: Arc<RwLock<Option<CarrierSignalSample>>>,
}

impl CarrierSignalHandle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&self) -> Option<CarrierSignalSample> {
        match self.inner.read() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn write(&self, sample: CarrierSignalSample) {
        let mut guard = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = Some(sample);
    }

    pub fn clear(&self) {
        let mut guard = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        *guard = None;
    }
}

pub fn write_status_file(path: &std::path::Path, sample: &CarrierSignalSample) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let json = serde_json::to_string_pretty(sample).map_err(std::io::Error::other)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json.as_bytes())?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::H256;

    fn sample() -> CarrierSignalSample {
        CarrierSignalSample {
            carrier_id: "7AY-Test".into(),
            region_id: H256::repeat_byte(0x11),
            rssi_dbm: -67,
            snr_db: 18,
            bars: 4,
            network_type: "LTE".into(),
            latency_ms: 42,
            jitter_ms: 3,
            packet_loss_pct: 0,
            captured_at: 1_700_000_000,
            sequence: 1,
        }
    }

    #[test]
    fn handle_round_trip() {
        let h = CarrierSignalHandle::new();
        assert!(h.read().is_none());
        h.write(sample());
        assert_eq!(h.read().unwrap().rssi_dbm, -67);
        h.clear();
        assert!(h.read().is_none());
    }

    #[test]
    fn writes_status_atomically() {
        let tmp = tempfile_path("carrier-signal-status");
        write_status_file(&tmp, &sample()).expect("write");
        let read = std::fs::read_to_string(&tmp).expect("read");
        assert!(read.contains("\"rssi_dbm\": -67"));
        std::fs::remove_file(&tmp).ok();
    }

    fn tempfile_path(label: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        p.push(format!("7ay-{label}-{pid}-{nanos}.json"));
        p
    }
}
