pub mod bluetooth;
pub mod inherent;
pub mod mock;
pub mod oui;
pub mod types;
pub mod wifi;

pub use inherent::{DeviceScanInherentDataProvider, ScanResultsHandle};
pub use mock::{MockConfig, MockScanner};
pub use types::*;

use bluetooth::BluetoothScanner;
use wifi::WifiScanner;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScannerMode {
    Real,
    Mock,
    Linux,
    Disabled,
}

impl Default for ScannerMode {
    fn default() -> Self {
        Self::Real
    }
}

impl std::str::FromStr for ScannerMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "real" => Ok(Self::Real),
            "mock" => Ok(Self::Mock),
            "linux" => Ok(Self::Linux),
            "disabled" | "off" | "none" => Ok(Self::Disabled),
            _ => Err(format!("Unknown scanner mode: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScannerConfig {
    pub mode: ScannerMode,
    pub scan_interval_secs: u64,
    pub wifi_enabled: bool,
    pub bluetooth_enabled: bool,
    pub max_devices_per_block: u32,
    pub reporter_position: Position,
    pub bt_scan_duration_secs: u64,
    pub mock_device_count: u32,
    pub mock_seed: u64,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            mode: ScannerMode::Real,
            scan_interval_secs: 6,
            wifi_enabled: true,
            bluetooth_enabled: false,
            max_devices_per_block: 100,
            reporter_position: Position::default(),
            bt_scan_duration_secs: 3,
            mock_device_count: 15,
            mock_seed: 42,
        }
    }
}

pub fn create_scan_results_handle() -> ScanResultsHandle {
    Arc::new(RwLock::new(ScanResults::default()))
}

pub async fn run_scanner(config: ScannerConfig, scan_results: ScanResultsHandle) {
    match config.mode {
        ScannerMode::Disabled => {
            log::info!("Device scanner disabled");
            return;
        }
        ScannerMode::Mock => {
            run_mock_scanner(config, scan_results).await;
        }
        ScannerMode::Real | ScannerMode::Linux => {
            run_real_scanner(config, scan_results).await;
        }
    }
}

async fn run_mock_scanner(config: ScannerConfig, scan_results: ScanResultsHandle) {
    let scan_interval = Duration::from_secs(config.scan_interval_secs);

    let mock_config = MockConfig {
        device_count: config.mock_device_count,
        position: config.reporter_position,
        seed: config.mock_seed,
        ..MockConfig::default()
    };

    let mut mock_scanner = MockScanner::new(mock_config);

    log::info!(
        "Mock scanner started - {} devices, interval: {}s",
        config.mock_device_count,
        config.scan_interval_secs
    );

    loop {
        let devices = mock_scanner.scan().await;

        {
            let mut guard = scan_results.write().await;
            guard.devices = devices.clone();
            guard.last_scan = Some(std::time::SystemTime::now());
        }

        log::info!("Mock scan complete: {} devices", devices.len());

        tokio::time::sleep(scan_interval).await;
    }
}

async fn run_real_scanner(config: ScannerConfig, scan_results: ScanResultsHandle) {
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
