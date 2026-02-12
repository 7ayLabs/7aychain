use super::types::{DeviceScanInherentData, Position, ScanResults};
use sp_inherents::{InherentData, InherentDataProvider, InherentIdentifier};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"devscan0";

pub type ScanResultsHandle = Arc<RwLock<ScanResults>>;

pub struct DeviceScanInherentDataProvider {
    scan_results: ScanResultsHandle,
    reporter_position: Position,
    max_devices: u32,
}

impl DeviceScanInherentDataProvider {
    pub fn new(scan_results: ScanResultsHandle, reporter_position: Position, max_devices: u32) -> Self {
        Self {
            scan_results,
            reporter_position,
            max_devices,
        }
    }
}

#[async_trait::async_trait]
impl InherentDataProvider for DeviceScanInherentDataProvider {
    async fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), sp_inherents::Error> {
        let results = self.scan_results.read().await;

        if results.devices.is_empty() {
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
