use super::types::{DetectedDeviceType, ScanResults, ScanSignalType, ScannedDevice};
use crate::scanner::ScanResultsHandle;
use serde::Deserialize;
use sp_core::{blake2_256, H256};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ExternalScannerConfig {
    pub scan_interval_secs: u64,
    pub scan_file: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct ExternalScanFile {
    generated_at: Option<u64>,
    devices: Vec<ExternalDeviceRecord>,
}

#[derive(Debug, Deserialize)]
struct ExternalDeviceRecord {
    mac_hash: String,
    rssi: i8,
    signal_type: String,
    #[serde(default)]
    device_type: Option<String>,
    #[serde(default)]
    vendor: Option<String>,
    #[serde(default)]
    device_name: Option<String>,
    #[serde(default)]
    frequency: Option<u16>,
    #[serde(default)]
    detected_at: Option<u64>,
}

pub async fn run_external_scanner(config: ExternalScannerConfig, scan_results: ScanResultsHandle) {
    let scan_interval = Duration::from_secs(config.scan_interval_secs);
    let Some(scan_file) = config.scan_file.clone() else {
        log::warn!(
            "External scanner selected without --external-scan-file; scanner will stay idle"
        );
        return;
    };

    let mut last_file_hash: Option<H256> = None;

    log::info!(
        "External scanner started - file: {}, interval: {}s",
        scan_file.display(),
        config.scan_interval_secs
    );

    loop {
        match std::fs::read(&scan_file) {
            Ok(bytes) => {
                let file_hash = H256(blake2_256(&bytes));
                if last_file_hash == Some(file_hash) {
                    tokio::time::sleep(scan_interval).await;
                    continue;
                }

                match parse_scan_file(&bytes) {
                    Ok(parsed) => {
                        {
                            let mut guard = scan_results.write().await;
                            guard.devices = parsed.devices;
                            guard.last_scan = Some(SystemTime::now());
                            guard.scan_sequence = guard.scan_sequence.saturating_add(1);
                        }
                        last_file_hash = Some(file_hash);
                    }
                    Err(error) => {
                        log::warn!(
                            "External scanner rejected {}: {}",
                            scan_file.display(),
                            error
                        );
                        clear_scan_results(&scan_results).await;
                    }
                }
            }
            Err(error) => {
                log::debug!(
                    "External scanner could not read {}: {}",
                    scan_file.display(),
                    error
                );
                clear_scan_results(&scan_results).await;
            }
        }

        tokio::time::sleep(scan_interval).await;
    }
}

fn parse_scan_file(bytes: &[u8]) -> Result<ScanResults, String> {
    let parsed: ExternalScanFile =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid JSON: {error}"))?;

    if parsed.devices.is_empty() {
        return Err("device list is empty".into());
    }

    let default_detected_at = parsed.generated_at.unwrap_or_else(current_unix_time);
    let devices = parsed
        .devices
        .into_iter()
        .map(|device| to_scanned_device(device, default_detected_at))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ScanResults {
        devices,
        last_scan: Some(SystemTime::now()),
        scan_sequence: 0,
        last_emitted_sequence: None,
    })
}

fn to_scanned_device(
    device: ExternalDeviceRecord,
    default_detected_at: u64,
) -> Result<ScannedDevice, String> {
    Ok(ScannedDevice {
        mac_hash: parse_h256(&device.mac_hash)?,
        rssi: device.rssi,
        signal_type: parse_signal_type(&device.signal_type)?,
        device_type: parse_device_type(device.device_type.as_deref().unwrap_or("unknown"))?,
        vendor: encode_optional::<32>(device.vendor.as_deref())?,
        device_name: encode_optional::<64>(device.device_name.as_deref())?,
        frequency: device.frequency,
        detected_at: device.detected_at.unwrap_or(default_detected_at),
    })
}

fn parse_h256(input: &str) -> Result<H256, String> {
    let trimmed = input.strip_prefix("0x").unwrap_or(input);
    if trimmed.len() != 64 {
        return Err(format!(
            "mac_hash must be 32 bytes, got {} hex chars",
            trimmed.len()
        ));
    }

    let mut bytes = [0u8; 32];
    hex::decode_to_slice(trimmed, &mut bytes)
        .map_err(|error| format!("invalid mac_hash hex: {error}"))?;
    Ok(H256(bytes))
}

fn parse_signal_type(input: &str) -> Result<ScanSignalType, String> {
    match input.to_ascii_lowercase().as_str() {
        "wifi" => Ok(ScanSignalType::Wifi),
        "bluetooth" => Ok(ScanSignalType::Bluetooth),
        "ble" => Ok(ScanSignalType::Ble),
        other => Err(format!("unsupported signal_type '{other}'")),
    }
}

fn parse_device_type(input: &str) -> Result<DetectedDeviceType, String> {
    match input.to_ascii_lowercase().as_str() {
        "unknown" => Ok(DetectedDeviceType::Unknown),
        "iphone" => Ok(DetectedDeviceType::IPhone),
        "android" => Ok(DetectedDeviceType::Android),
        "macbook" => Ok(DetectedDeviceType::MacBook),
        "windowspc" | "windows_pc" | "windows" => Ok(DetectedDeviceType::WindowsPC),
        "linuxpc" | "linux_pc" | "linux" => Ok(DetectedDeviceType::LinuxPC),
        "ipad" => Ok(DetectedDeviceType::IPad),
        "applewatch" | "apple_watch" => Ok(DetectedDeviceType::AppleWatch),
        "airpods" => Ok(DetectedDeviceType::AirPods),
        "smarttv" | "smart_tv" => Ok(DetectedDeviceType::SmartTV),
        "iotdevice" | "iot_device" | "iot" => Ok(DetectedDeviceType::IoTDevice),
        "networkdevice" | "network_device" => Ok(DetectedDeviceType::NetworkDevice),
        "printer" => Ok(DetectedDeviceType::Printer),
        "gameconsole" | "game_console" => Ok(DetectedDeviceType::GameConsole),
        other => Err(format!("unsupported device_type '{other}'")),
    }
}

fn encode_optional<const N: usize>(value: Option<&str>) -> Result<Option<[u8; N]>, String> {
    let Some(value) = value else {
        return Ok(None);
    };

    let bytes = value.as_bytes();
    if bytes.len() > N {
        return Err(format!("value '{}' exceeds {} bytes", value, N));
    }

    let mut fixed = [0u8; N];
    fixed[..bytes.len()].copy_from_slice(bytes);
    Ok(Some(fixed))
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

async fn clear_scan_results(scan_results: &ScanResultsHandle) {
    let mut guard = scan_results.write().await;
    guard.devices.clear();
    guard.last_scan = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_external_scan_file() {
        let json = br#"{
            "generated_at": 1700000000,
            "devices": [
                {
                    "mac_hash": "0x1111111111111111111111111111111111111111111111111111111111111111",
                    "rssi": -42,
                    "signal_type": "wifi",
                    "device_type": "iphone",
                    "vendor": "apple",
                    "device_name": "alice-phone",
                    "frequency": 2412
                }
            ]
        }"#;

        let parsed = parse_scan_file(json).expect("file parses");
        assert_eq!(parsed.devices.len(), 1);
        assert_eq!(parsed.devices[0].rssi, -42);
        assert_eq!(parsed.devices[0].signal_type, ScanSignalType::Wifi);
        assert_eq!(parsed.devices[0].device_type, DetectedDeviceType::IPhone);
    }

    #[test]
    fn rejects_invalid_hash_length() {
        let json = br#"{
            "devices": [
                {
                    "mac_hash": "0x1234",
                    "rssi": -42,
                    "signal_type": "wifi"
                }
            ]
        }"#;

        let error = parse_scan_file(json).expect_err("parse should fail");
        assert!(error.contains("mac_hash"));
    }

    #[test]
    fn rejects_oversized_device_name() {
        let json = br#"{
            "devices": [
                {
                    "mac_hash": "0x1111111111111111111111111111111111111111111111111111111111111111",
                    "rssi": -42,
                    "signal_type": "ble",
                    "device_name": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                }
            ]
        }"#;

        let error = parse_scan_file(json).expect_err("parse should fail");
        assert!(error.contains("exceeds 64 bytes"));
    }
}
