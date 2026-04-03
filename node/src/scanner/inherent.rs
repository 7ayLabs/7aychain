use super::types::{DeviceScanInherentData, Position, ScanResults};
use sp_inherents::{InherentData, InherentDataProvider, InherentIdentifier};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"devscan0";

pub type ScanResultsHandle = Arc<RwLock<ScanResults>>;

pub struct DeviceScanInherentDataProvider {
    scan_results: ScanResultsHandle,
    reporter_position: Position,
    max_devices: u32,
    max_scan_age: Duration,
}

impl DeviceScanInherentDataProvider {
    pub fn new(
        scan_results: ScanResultsHandle,
        reporter_position: Position,
        max_devices: u32,
        max_scan_age_secs: u64,
    ) -> Self {
        Self {
            scan_results,
            reporter_position,
            max_devices,
            max_scan_age: Duration::from_secs(max_scan_age_secs),
        }
    }
}

#[async_trait::async_trait]
impl InherentDataProvider for DeviceScanInherentDataProvider {
    async fn provide_inherent_data(
        &self,
        inherent_data: &mut InherentData,
    ) -> Result<(), sp_inherents::Error> {
        let mut results = self.scan_results.write().await;

        if results.devices.is_empty() {
            return Ok(());
        }

        let Some(last_scan) = results.last_scan else {
            return Ok(());
        };

        let age = SystemTime::now()
            .duration_since(last_scan)
            .unwrap_or_else(|_| Duration::from_secs(0));
        if age > self.max_scan_age {
            log::warn!(
                "Skipping stale device scan data (age: {}s, max: {}s)",
                age.as_secs(),
                self.max_scan_age.as_secs(),
            );
            return Ok(());
        }

        if results.last_emitted_sequence == Some(results.scan_sequence) {
            return Ok(());
        }

        let scan_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let devices: Vec<_> = results
            .devices
            .iter()
            .take(self.max_devices as usize)
            .cloned()
            .collect();

        let data = DeviceScanInherentData {
            devices,
            reporter_position: self.reporter_position,
            scan_timestamp,
        };

        inherent_data.put_data(INHERENT_IDENTIFIER, &data)?;
        results.last_emitted_sequence = Some(results.scan_sequence);
        Ok(())
    }

    async fn try_handle_error(
        &self,
        _identifier: &InherentIdentifier,
        _error: &[u8],
    ) -> Option<Result<(), sp_inherents::Error>> {
        Some(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::types::{DetectedDeviceType, ScanSignalType, ScannedDevice};
    use sp_core::H256;

    #[tokio::test]
    async fn provider_emits_each_scan_only_once() {
        let handle = Arc::new(RwLock::new(ScanResults::default()));
        {
            let mut guard = handle.write().await;
            guard.devices = vec![ScannedDevice {
                mac_hash: H256::repeat_byte(1),
                rssi: -45,
                signal_type: ScanSignalType::Wifi,
                device_type: DetectedDeviceType::IPhone,
                vendor: None,
                device_name: None,
                frequency: Some(2412),
                detected_at: 1,
            }];
            guard.last_scan = Some(SystemTime::now());
            guard.scan_sequence = 1;
        }

        let provider =
            DeviceScanInherentDataProvider::new(handle.clone(), Position::default(), 10, 30);

        let mut first = InherentData::new();
        provider
            .provide_inherent_data(&mut first)
            .await
            .expect("first provide succeeds");
        assert!(matches!(
            first.get_data::<DeviceScanInherentData>(&INHERENT_IDENTIFIER),
            Ok(Some(_))
        ));

        let mut second = InherentData::new();
        provider
            .provide_inherent_data(&mut second)
            .await
            .expect("second provide succeeds");
        assert!(matches!(
            second.get_data::<DeviceScanInherentData>(&INHERENT_IDENTIFIER),
            Ok(None)
        ));
    }

    #[tokio::test]
    async fn provider_rejects_stale_scan_data() {
        let handle = Arc::new(RwLock::new(ScanResults::default()));
        {
            let mut guard = handle.write().await;
            guard.devices = vec![ScannedDevice {
                mac_hash: H256::repeat_byte(2),
                rssi: -60,
                signal_type: ScanSignalType::Ble,
                device_type: DetectedDeviceType::Unknown,
                vendor: None,
                device_name: None,
                frequency: None,
                detected_at: 1,
            }];
            guard.last_scan = Some(SystemTime::now() - Duration::from_secs(61));
            guard.scan_sequence = 7;
        }

        let provider = DeviceScanInherentDataProvider::new(handle, Position::default(), 10, 30);
        let mut data = InherentData::new();
        provider
            .provide_inherent_data(&mut data)
            .await
            .expect("provide succeeds");

        assert!(matches!(
            data.get_data::<DeviceScanInherentData>(&INHERENT_IDENTIFIER),
            Ok(None)
        ));
    }
}
