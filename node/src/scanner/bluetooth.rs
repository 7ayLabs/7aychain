use super::oui::{infer_device_type_from_name, lookup_device_type, lookup_vendor, name_to_bytes, vendor_to_bytes};
use super::types::{DetectedDeviceType, ScanError, ScanSignalType, ScannedDevice};
use sp_core::{blake2_256, H256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(feature = "bluetooth")]
use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter};
#[cfg(feature = "bluetooth")]
use btleplug::platform::Manager;

pub struct BluetoothScanner {
    #[cfg(feature = "bluetooth")]
    manager: Option<Manager>,
    #[cfg(not(feature = "bluetooth"))]
    _phantom: std::marker::PhantomData<()>,
}

impl BluetoothScanner {
    pub async fn new() -> Result<Self, ScanError> {
        #[cfg(feature = "bluetooth")]
        {
            let manager = Manager::new()
                .await
                .map_err(|e| ScanError::BluetoothError(e.to_string()))?;
            Ok(Self {
                manager: Some(manager),
            })
        }

        #[cfg(not(feature = "bluetooth"))]
        {
            log::warn!("Bluetooth scanning disabled - btleplug feature not enabled");
            Ok(Self {
                _phantom: std::marker::PhantomData,
            })
        }
    }

    pub async fn scan(&self, scan_duration: Duration) -> Result<Vec<ScannedDevice>, ScanError> {
        #[cfg(feature = "bluetooth")]
        {
            let manager = self
                .manager
                .as_ref()
                .ok_or_else(|| ScanError::BluetoothError("Manager not initialized".to_string()))?;

            let adapters = manager
                .adapters()
                .await
                .map_err(|e| ScanError::BluetoothError(e.to_string()))?;

            let adapter = adapters
                .into_iter()
                .next()
                .ok_or(ScanError::InterfaceNotFound)?;

            adapter
                .start_scan(ScanFilter::default())
                .await
                .map_err(|e| ScanError::BluetoothError(e.to_string()))?;

            tokio::time::sleep(scan_duration).await;

            adapter
                .stop_scan()
                .await
                .map_err(|e| ScanError::BluetoothError(e.to_string()))?;

            let peripherals = adapter
                .peripherals()
                .await
                .map_err(|e| ScanError::BluetoothError(e.to_string()))?;

            let mut devices = Vec::new();
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            for peripheral in peripherals {
                if let Ok(Some(props)) = peripheral.properties().await {
                    let device = self.process_peripheral(props, now);
                    devices.push(device);
                }
            }

            Ok(devices)
        }

        #[cfg(not(feature = "bluetooth"))]
        {
            let _ = scan_duration;
            Ok(Vec::new())
        }
    }

    #[cfg(feature = "bluetooth")]
    fn process_peripheral(
        &self,
        props: btleplug::api::PeripheralProperties,
        timestamp: u64,
    ) -> ScannedDevice {
        let mac_bytes: [u8; 6] = props.address.into_inner();
        let mac_hash = H256(blake2_256(&mac_bytes));
        let oui: [u8; 3] = [mac_bytes[0], mac_bytes[1], mac_bytes[2]];

        let vendor = lookup_vendor(&oui);
        let mut device_type = lookup_device_type(&oui);

        if let Some(ref name) = props.local_name {
            let name_inferred = infer_device_type_from_name(name);
            if name_inferred != DetectedDeviceType::Unknown {
                device_type = name_inferred;
            }
        }

        if device_type == DetectedDeviceType::Unknown {
            // manufacturer_data is a HashMap, not Option
            if props.manufacturer_data.contains_key(&0x004C) {
                // Apple company ID
                device_type = DetectedDeviceType::IPhone;
            } else if props.manufacturer_data.contains_key(&0x0075) {
                // Samsung company ID
                device_type = DetectedDeviceType::Android;
            }
        }

        let signal_type = if props.services.iter().any(|s| {
            let uuid_str = s.to_string().to_lowercase();
            uuid_str.contains("180f") || uuid_str.contains("180a") || uuid_str.contains("1812")
        }) {
            ScanSignalType::Ble
        } else {
            ScanSignalType::Bluetooth
        };

        ScannedDevice {
            mac_hash,
            rssi: props.rssi.unwrap_or(-127) as i8,
            signal_type,
            device_type,
            vendor: vendor.map(vendor_to_bytes),
            device_name: props.local_name.as_ref().map(|n| name_to_bytes(n)),
            frequency: None,
            detected_at: timestamp,
        }
    }
}
