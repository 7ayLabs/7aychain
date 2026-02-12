pub mod bluetooth;
pub mod inherent;
pub mod oui;
pub mod types;
pub mod wifi;

pub use inherent::{DeviceScanInherentDataProvider, ScanResultsHandle};
pub use types::*;

use bluetooth::BluetoothScanner;
use wifi::WifiScanner;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ScannerConfig {
    pub scan_interval_secs: u64,
    pub wifi_enabled: bool,
    pub bluetooth_enabled: bool,
    pub max_devices_per_block: u32,
    pub reporter_position: Position,
    pub bt_scan_duration_secs: u64,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            scan_interval_secs: 6,
            wifi_enabled: true,
            // Bluetooth disabled by default - btleplug can crash on some macOS versions
            // Enable with --scanner-bluetooth flag when ready
            bluetooth_enabled: false,
            max_devices_per_block: 100,
            reporter_position: Position::default(),
            bt_scan_duration_secs: 3,
        }
    }
}

pub fn create_scan_results_handle() -> ScanResultsHandle {
    Arc::new(RwLock::new(ScanResults::default()))
}

pub async fn run_scanner(config: ScannerConfig, scan_results: ScanResultsHandle) {
    let scan_interval = Duration::from_secs(config.scan_interval_secs);
    let bt_scan_duration = Duration::from_secs(config.bt_scan_duration_secs);

    let wifi_scanner = if config.wifi_enabled {
        Some(WifiScanner::new())
    } else {
        None
    };

    let bt_scanner = if config.bluetooth_enabled {
        match BluetoothScanner::new().await {
            Ok(scanner) => Some(scanner),
            Err(e) => {
                log::warn!("Failed to initialize Bluetooth scanner: {}", e);
                None
            }
        }
    } else {
        None
    };

    log::info!(
        "Device scanner started - WiFi: {}, Bluetooth: {}, Interval: {}s",
        wifi_scanner.is_some(),
        bt_scanner.is_some(),
        config.scan_interval_secs
    );

    loop {
        let mut devices = Vec::new();

        // WiFi scanning with error recovery
        if let Some(ref scanner) = wifi_scanner {
            match scanner.scan().await {
                Ok(wifi_devices) => {
                    log::info!("WiFi scan found {} devices", wifi_devices.len());
                    devices.extend(wifi_devices);
                }
                Err(e) => {
                    log::warn!("WiFi scan failed: {}", e);
                }
            }
        }

        // Bluetooth scanning with error recovery
        if let Some(ref scanner) = bt_scanner {
            match scanner.scan(bt_scan_duration).await {
                Ok(bt_devices) => {
                    log::info!("Bluetooth scan found {} devices", bt_devices.len());
                    devices.extend(bt_devices);
                }
                Err(e) => {
                    log::warn!("Bluetooth scan failed: {}", e);
                }
            }
        }

        devices.truncate(config.max_devices_per_block as usize);

        // Update shared state
        {
            let mut guard = scan_results.write().await;
            guard.devices = devices.clone();
            guard.last_scan = Some(std::time::SystemTime::now());
        }

        if !devices.is_empty() {
            log::info!(
                "Scan complete: {} devices ready for next block",
                devices.len()
            );
        }

        tokio::time::sleep(scan_interval).await;
    }
}

pub fn start_scanner_task(
    task_manager: &sc_service::TaskManager,
    config: ScannerConfig,
    scan_results: ScanResultsHandle,
) {
    let config_clone = config.clone();
    let results_clone = scan_results.clone();

    task_manager.spawn_handle().spawn(
        "device-scanner",
        Some("scanner"),
        async move {
            run_scanner(config_clone, results_clone).await;
        },
    );

    log::info!("Device scanner task spawned");
}
