use super::oui::{infer_device_type_from_name, lookup_device_type, lookup_vendor, name_to_bytes, vendor_to_bytes};
use super::types::{ScanError, ScanSignalType, ScannedDevice};
use sp_core::{blake2_256, H256};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct WifiScanner {
    interface_name: Option<String>,
}

impl WifiScanner {
    pub fn new() -> Self {
        Self {
            interface_name: None,
        }
    }

    pub fn with_interface(interface: &str) -> Self {
        Self {
            interface_name: Some(interface.to_string()),
        }
    }

    #[cfg(target_os = "macos")]
    pub async fn scan(&self) -> Result<Vec<ScannedDevice>, ScanError> {
        use std::process::Command;

        // Try the airport utility first (older macOS versions)
        let airport_path = "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport";

        if std::path::Path::new(airport_path).exists() {
            let output = Command::new(airport_path)
                .arg("-s")
                .output()
                .map_err(|e| ScanError::ScanFailed(format!("Failed to run airport: {}", e)))?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                return Ok(self.parse_airport_output(&stdout));
            }
        }

        // Fall back to system_profiler for newer macOS versions
        let output = Command::new("/usr/sbin/system_profiler")
            .args(["SPAirPortDataType", "-json"])
            .output()
            .map_err(|e| ScanError::ScanFailed(format!("Failed to run system_profiler: {}", e)))?;

        if !output.status.success() {
            return Err(ScanError::ScanFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let devices = self.parse_system_profiler_output(&stdout);
        Ok(devices)
    }

    #[cfg(not(target_os = "macos"))]
    pub async fn scan(&self) -> Result<Vec<ScannedDevice>, ScanError> {
        Err(ScanError::UnsupportedPlatform)
    }

    #[cfg(target_os = "macos")]
    fn parse_system_profiler_output(&self, output: &str) -> Vec<ScannedDevice> {
        let mut devices = Vec::new();
        let mut total_networks = 0usize;
        let mut redacted_count = 0usize;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Parse the JSON output from system_profiler
        // Structure: SPAirPortDataType[0].spairport_airport_interfaces[0].spairport_airport_other_local_wireless_networks
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            if let Some(airport_data) = json.get("SPAirPortDataType").and_then(|v| v.as_array()) {
                for data in airport_data {
                    // Navigate to spairport_airport_interfaces
                    if let Some(interfaces) = data
                        .get("spairport_airport_interfaces")
                        .and_then(|v| v.as_array())
                    {
                        for interface in interfaces {
                            // Get the networks list
                            if let Some(networks) = interface
                                .get("spairport_airport_other_local_wireless_networks")
                                .and_then(|v| v.as_array())
                            {
                                total_networks += networks.len();
                                for network in networks {
                                    // Count redacted networks
                                    if let Some(name) = network.get("_name").and_then(|v| v.as_str()) {
                                        if name == "<redacted>" {
                                            redacted_count += 1;
                                            continue;
                                        }
                                    }
                                    if let Some(device) = self.parse_network_entry(network, now) {
                                        devices.push(device);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Log macOS privacy limitation if networks are being redacted
        if redacted_count > 0 && devices.is_empty() {
            log::debug!(
                "WiFi: {} networks found but {} redacted by macOS privacy. \
                 Run with elevated privileges or use Location Services.",
                total_networks,
                redacted_count
            );
        }

        devices
    }

    #[cfg(target_os = "macos")]
    fn parse_network_entry(&self, network: &serde_json::Value, timestamp: u64) -> Option<ScannedDevice> {
        let ssid = network.get("_name").and_then(|v| v.as_str()).unwrap_or("");

        // Skip empty/hidden SSIDs
        if ssid.is_empty() || ssid == "<redacted>" {
            return None;
        }

        // Try to get BSSID (may not be available on newer macOS for privacy)
        let bssid_opt = network.get("spairport_network_bssid").and_then(|v| v.as_str());

        let rssi: i8 = network
            .get("spairport_signal_noise")
            .and_then(|v| v.as_str())
            .and_then(|s| s.split('/').next())
            .and_then(|s| s.trim().replace(" dBm", "").parse().ok())
            .unwrap_or(-100);

        let channel: u16 = network
            .get("spairport_network_channel")
            .and_then(|v| v.as_str())
            .and_then(|s| s.split_whitespace().next())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);

        let frequency = channel_to_frequency(channel);

        // Use BSSID if available, otherwise hash the SSID
        let (mac_hash, vendor, device_type) = if let Some(bssid) = bssid_opt {
            if let Some(mac_bytes) = parse_bssid(bssid) {
                let oui: [u8; 3] = [mac_bytes[0], mac_bytes[1], mac_bytes[2]];
                (
                    H256(blake2_256(&mac_bytes)),
                    lookup_vendor(&oui),
                    lookup_device_type(&oui),
                )
            } else {
                // Fallback to SSID hash
                (H256(blake2_256(ssid.as_bytes())), None, infer_device_type_from_name(ssid))
            }
        } else {
            // No BSSID available (macOS privacy) - use SSID hash
            (H256(blake2_256(ssid.as_bytes())), None, infer_device_type_from_name(ssid))
        };

        Some(ScannedDevice {
            mac_hash,
            rssi,
            signal_type: ScanSignalType::Wifi,
            device_type,
            vendor: vendor.map(vendor_to_bytes),
            device_name: Some(name_to_bytes(ssid)),
            frequency: Some(frequency),
            detected_at: timestamp,
        })
    }

    #[cfg(target_os = "macos")]
    fn parse_airport_output(&self, output: &str) -> Vec<ScannedDevice> {
        let mut devices = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for line in output.lines().skip(1) {
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 7 {
                continue;
            }

            let ssid = parts[0];
            let bssid = parts[1];
            let rssi: i8 = parts[2].parse().unwrap_or(-100);
            let channel: u16 = parts[3].parse().unwrap_or(0);

            let frequency = channel_to_frequency(channel);

            if let Some(mac_bytes) = parse_bssid(bssid) {
                let mac_hash = H256(blake2_256(&mac_bytes));
                let oui: [u8; 3] = [mac_bytes[0], mac_bytes[1], mac_bytes[2]];

                let vendor = lookup_vendor(&oui);
                let mut device_type = lookup_device_type(&oui);

                let name_inferred = infer_device_type_from_name(ssid);
                if name_inferred != super::types::DetectedDeviceType::Unknown {
                    device_type = name_inferred;
                }

                devices.push(ScannedDevice {
                    mac_hash,
                    rssi,
                    signal_type: ScanSignalType::Wifi,
                    device_type,
                    vendor: vendor.map(vendor_to_bytes),
                    device_name: Some(name_to_bytes(ssid)),
                    frequency: Some(frequency),
                    detected_at: now,
                });
            }
        }

        devices
    }
}

fn parse_bssid(bssid: &str) -> Option<[u8; 6]> {
    let parts: Vec<&str> = bssid.split(':').collect();
    if parts.len() != 6 {
        return None;
    }

    let mut bytes = [0u8; 6];
    for (i, part) in parts.iter().enumerate() {
        bytes[i] = u8::from_str_radix(part, 16).ok()?;
    }
    Some(bytes)
}

fn channel_to_frequency(channel: u16) -> u16 {
    match channel {
        1 => 2412,
        2 => 2417,
        3 => 2422,
        4 => 2427,
        5 => 2432,
        6 => 2437,
        7 => 2442,
        8 => 2447,
        9 => 2452,
        10 => 2457,
        11 => 2462,
        12 => 2467,
        13 => 2472,
        14 => 2484,
        36 => 5180,
        40 => 5200,
        44 => 5220,
        48 => 5240,
        52 => 5260,
        56 => 5280,
        60 => 5300,
        64 => 5320,
        100 => 5500,
        104 => 5520,
        108 => 5540,
        112 => 5560,
        116 => 5580,
        120 => 5600,
        124 => 5620,
        128 => 5640,
        132 => 5660,
        136 => 5680,
        140 => 5700,
        144 => 5720,
        149 => 5745,
        153 => 5765,
        157 => 5785,
        161 => 5805,
        165 => 5825,
        _ => 0,
    }
}
