//! Device scanning module for 7aychain presence verification.
//!
//! # Architecture (v0.8.11+)
//!
//! The scanner system has transitioned from WiFi/Bluetooth hardware scanning
//! to network latency-based presence verification:
//!
//! ## New Architecture: Presence-Based Triangulation (PBT)
//!
//! 1. **Position Claims**: Nodes claim their geographic position
//! 2. **Witness Attestations**: Validators attest to other nodes' presence
//!    using network RTT measurements
//! 3. **Triangulation**: Multiple witness attestations triangulate a node's
//!    position using weighted latency circles
//!
//! ## Benefits of PBT over Hardware Scanning
//!
//! - **Privacy-safe**: No scanning of external user devices
//! - **No hardware requirements**: Works on any network-connected node
//! - **Cross-platform**: Same implementation on all platforms
//! - **Decentralized**: Validators verify each other, no central authority
//!
//! ## Deprecated Modules
//!
//! The following modules are deprecated but retained for backward compatibility:
//! - `wifi` - WiFi device scanning (use `latency` instead)
//! - `bluetooth` - Bluetooth device scanning (use `latency` instead)
//!
//! ## Usage
//!
//! ```ignore
//! // Default: Network latency-based scanning
//! let config = ScannerConfig { mode: ScannerMode::Latency, ..Default::default() };
//! ```

#[deprecated(
    since = "0.8.11",
    note = "Use latency-based scanning instead of WiFi/Bluetooth"
)]
pub mod bluetooth;
pub mod inherent;
pub mod latency;
pub mod mock;
pub mod oui;
pub mod types;
#[deprecated(
    since = "0.8.11",
    note = "Use latency-based scanning instead of WiFi/Bluetooth"
)]
pub mod wifi;

pub use inherent::{DeviceScanInherentDataProvider, ScanResultsHandle};
pub use latency::{LatencyScanner, LatencyScannerConfig, PeerLatency, LatencyStatistics};
pub use mock::{MockConfig, MockScanner};
pub use types::*;

#[allow(deprecated)]
use bluetooth::BluetoothScanner;
#[allow(deprecated)]
use wifi::WifiScanner;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Scanner mode for presence verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScannerMode {
    /// Network latency-based scanning (recommended - no hardware required)
    Latency,
    /// Real WiFi/Bluetooth scanning (deprecated - requires hardware)
    #[deprecated(note = "Use Latency mode instead for privacy-safe scanning")]
    Real,
    /// Mock scanning for testing
    Mock,
    /// Linux-specific scanning (deprecated)
    #[deprecated(note = "Use Latency mode instead")]
    Linux,
    /// Scanner disabled
    Disabled,
}

impl Default for ScannerMode {
    fn default() -> Self {
        Self::Latency
    }
}

impl std::str::FromStr for ScannerMode {
    type Err = String;

    #[allow(deprecated)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "latency" | "network" => Ok(Self::Latency),
            "real" | "wifi" | "bluetooth" => Ok(Self::Real),
            "mock" => Ok(Self::Mock),
            "linux" => Ok(Self::Linux),
            "disabled" | "off" | "none" => Ok(Self::Disabled),
            _ => Err(format!("Unknown scanner mode: {}", s)),
        }
    }
}

impl ScannerMode {
    /// Whether this mode requires special hardware.
    #[allow(deprecated)]
    pub fn requires_hardware(&self) -> bool {
        matches!(self, Self::Real | Self::Linux)
    }

    /// Whether this mode is privacy-safe (doesn't scan external devices).
    pub fn is_privacy_safe(&self) -> bool {
        matches!(self, Self::Latency | Self::Mock | Self::Disabled)
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

#[allow(deprecated)]
pub async fn run_scanner(config: ScannerConfig, scan_results: ScanResultsHandle) {
    match config.mode {
        ScannerMode::Disabled => {
            log::info!("Device scanner disabled");
            return;
        }
        ScannerMode::Mock => {
            run_mock_scanner(config, scan_results).await;
        }
        ScannerMode::Latency => {
            run_latency_scanner(config, scan_results).await;
        }
        ScannerMode::Real | ScannerMode::Linux => {
            log::warn!("WiFi/Bluetooth scanning is deprecated. Consider using --scanner-mode=latency");
            run_real_scanner(config, scan_results).await;
        }
    }
}

async fn run_latency_scanner(config: ScannerConfig, scan_results: ScanResultsHandle) {
    let scan_interval = Duration::from_secs(config.scan_interval_secs);

    log::info!(
        "Latency-based scanner started - Interval: {}s, Position: ({}, {}, {})",
        config.scan_interval_secs,
        config.reporter_position.x,
        config.reporter_position.y,
        config.reporter_position.z
    );

    // In a real implementation, this would integrate with the network layer
    // to measure RTT to connected peers. For now, we log that it's running.
    loop {
        // The latency measurements will come from the network layer via callbacks.
        // This loop just keeps the scanner task alive and logs status.
        {
            let guard = scan_results.read().await;
            log::debug!(
                "Latency scanner active - {} measurements",
                guard.devices.len()
            );
        }

        tokio::time::sleep(scan_interval).await;
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
