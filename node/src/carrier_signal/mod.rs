//! Per-node carrier signal subsystem.
//!
//! Beta scope: when the node runs, this task periodically samples a
//! carrier-signal source (file written by an external bridge, or a
//! synthetic mock for devnet) and exposes the latest sample through:
//!
//!   * the `seveny_currentCarrierSignal` JSON-RPC method, and
//!   * an atomic status file on disk (default `data/carrier-signal/status.json`).
//!
//! It is intentionally NOT wired to chain submission yet — the Python
//! bridge in `devnet/scripts/carrier_signal_publisher.py` reads the
//! status file and posts the appropriate extrinsics to `pallet-carrier`.
//! This separation lets non-validator nodes also "get signal" without
//! requiring a keystore.

mod file;
mod mock;
mod state;
mod types;

pub use state::CarrierSignalHandle;
pub use types::{CarrierSignalSample, MAX_CARRIER_ID_LEN, MAX_NETWORK_TYPE_LEN};

use std::path::PathBuf;
use std::time::Duration;

use sp_core::H256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarrierSignalMode {
    Disabled,
    File,
    Mock,
}

impl Default for CarrierSignalMode {
    fn default() -> Self {
        Self::Disabled
    }
}

impl std::str::FromStr for CarrierSignalMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "disabled" | "off" | "none" => Ok(Self::Disabled),
            "file" | "external" => Ok(Self::File),
            "mock" | "synthetic" => Ok(Self::Mock),
            other => Err(format!(
                "unknown carrier-signal mode '{other}'. valid: disabled, file, mock"
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CarrierSignalConfig {
    pub mode: CarrierSignalMode,
    pub interval_secs: u64,
    pub input_file: Option<PathBuf>,
    pub status_file: Option<PathBuf>,
    pub carrier_id: String,
    pub region_id: H256,
    pub network_type: String,
    pub mock_seed: u64,
}

impl Default for CarrierSignalConfig {
    fn default() -> Self {
        Self {
            mode: CarrierSignalMode::Disabled,
            interval_secs: 10,
            input_file: None,
            status_file: Some(PathBuf::from("data/carrier-signal/status.json")),
            carrier_id: "7AY-Test".into(),
            region_id: H256::repeat_byte(0x11),
            network_type: "LTE".into(),
            mock_seed: 42,
        }
    }
}

pub fn start_carrier_signal_task(
    task_manager: &sc_service::TaskManager,
    config: CarrierSignalConfig,
    handle: CarrierSignalHandle,
) {
    let handle_clone = handle.clone();
    task_manager
        .spawn_handle()
        .spawn("carrier-signal", Some("carrier"), async move {
            run(config, handle_clone).await;
        });
    log::info!("carrier-signal: task spawned");
}

async fn run(config: CarrierSignalConfig, handle: CarrierSignalHandle) {
    let interval = Duration::from_secs(config.interval_secs.max(1));
    match config.mode {
        CarrierSignalMode::Disabled => {
            log::info!("carrier-signal: disabled");
        }
        CarrierSignalMode::File => {
            let Some(input_file) = config.input_file.clone() else {
                log::warn!(
                    "carrier-signal: file mode requested but --carrier-signal-file is missing"
                );
                return;
            };
            file::run_file_source(
                file::FileSourceConfig {
                    input_file,
                    status_file: config.status_file.clone(),
                    interval,
                },
                handle,
            )
            .await;
        }
        CarrierSignalMode::Mock => {
            mock::run_mock_source(
                mock::MockSourceConfig {
                    carrier_id: config.carrier_id.clone(),
                    region_id: config.region_id,
                    network_type: config.network_type.clone(),
                    status_file: config.status_file.clone(),
                    interval,
                    seed: config.mock_seed,
                },
                handle,
            )
            .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_parses() {
        assert_eq!("mock".parse::<CarrierSignalMode>(), Ok(CarrierSignalMode::Mock));
        assert_eq!("File".parse::<CarrierSignalMode>(), Ok(CarrierSignalMode::File));
        assert_eq!("off".parse::<CarrierSignalMode>(), Ok(CarrierSignalMode::Disabled));
        assert!("nonsense".parse::<CarrierSignalMode>().is_err());
    }
}
