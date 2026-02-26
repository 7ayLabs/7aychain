pub mod inherent;
pub mod latency;
pub mod mock;
pub mod types;

pub use inherent::{DeviceScanInherentDataProvider, ScanResultsHandle};
pub use mock::{MockConfig, MockScanner};
pub use types::*;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScannerMode {
    Latency,
    Mock,
    Disabled,
}

impl Default for ScannerMode {
    fn default() -> Self {
        Self::Latency
    }
}

impl std::str::FromStr for ScannerMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "latency" | "network" => Ok(Self::Latency),
            "mock" => Ok(Self::Mock),
            "disabled" | "off" | "none" => Ok(Self::Disabled),
            other => Err(format!(
                "Unknown scanner mode '{}'. Valid: latency, mock, disabled",
                other
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScannerConfig {
    pub mode: ScannerMode,
    pub scan_interval_secs: u64,
    pub max_devices_per_block: u32,
    pub reporter_position: Position,
    pub mock_device_count: u32,
    pub mock_seed: u64,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            mode: ScannerMode::Latency,
            scan_interval_secs: 6,
            max_devices_per_block: 100,
            reporter_position: Position::default(),
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
        }
        ScannerMode::Mock => {
            run_mock_scanner(config, scan_results).await;
        }
        ScannerMode::Latency => {
            run_latency_scanner(config, scan_results).await;
        }
    }
}

async fn run_latency_scanner(config: ScannerConfig, scan_results: ScanResultsHandle) {
    let scan_interval = Duration::from_secs(config.scan_interval_secs);
    let mut scanner = latency::LatencyScanner::new();
    let mut mock_source =
        latency::MockLatencyScanner::new(50, 15, config.mock_seed);

    log::info!(
        "Latency-based scanner started - Interval: {}s, Position: ({}, {}, {})",
        config.scan_interval_secs,
        config.reporter_position.x,
        config.reporter_position.y,
        config.reporter_position.z
    );

    loop {
        // Generate latency measurements and feed them into the scanner
        let measurements =
            mock_source.generate_measurements(config.mock_device_count as usize);
        for (peer_id, latency) in &measurements {
            scanner.update_peer_latency(peer_id.clone(), latency.rtt_ms, latency.direct);
        }

        let stats = scanner.get_statistics();

        // Convert latency peers into ScannedDevice entries for the inherent
        let devices: Vec<ScannedDevice> = measurements
            .iter()
            .filter(|(_, lat)| lat.is_valid())
            .map(|(peer_id, lat)| {
                let mac_hash = sp_core::H256(sp_core::blake2_256(
                    &peer_id.to_bytes(),
                ));
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                ScannedDevice {
                    mac_hash,
                    rssi: -(lat.rtt_ms.min(127) as i8),
                    signal_type: ScanSignalType::default(),
                    device_type: DetectedDeviceType::NetworkDevice,
                    vendor: None,
                    device_name: None,
                    frequency: None,
                    detected_at: now,
                }
            })
            .collect();

        {
            let mut guard = scan_results.write().await;
            guard.devices = devices.clone();
            guard.last_scan = Some(std::time::SystemTime::now());
        }

        log::info!(
            "Latency scan complete: {} peers, avg_rtt={}ms, {} devices",
            stats.peer_count,
            stats.avg_rtt_ms,
            devices.len()
        );

        tokio::time::sleep(scan_interval).await;
    }
}

async fn run_mock_scanner(config: ScannerConfig, scan_results: ScanResultsHandle) {
    let scan_interval = Duration::from_secs(config.scan_interval_secs);

    let mock_config = MockConfig {
        device_count: config.mock_device_count,
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

pub fn start_scanner_task(
    task_manager: &sc_service::TaskManager,
    config: ScannerConfig,
    scan_results: ScanResultsHandle,
) {
    let config_clone = config.clone();
    let results_clone = scan_results.clone();

    task_manager
        .spawn_handle()
        .spawn("device-scanner", Some("scanner"), async move {
            run_scanner(config_clone, results_clone).await;
        });

    log::info!("Device scanner task spawned");
}
