use parity_scale_codec::{Decode, Encode};
use sp_core::H256;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub enum ScanSignalType {
    Wifi,
    Bluetooth,
    Ble,
}

impl Default for ScanSignalType {
    fn default() -> Self {
        Self::Wifi
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub enum DetectedDeviceType {
    Unknown,
    IPhone,
    Android,
    MacBook,
    WindowsPC,
    LinuxPC,
    IPad,
    AppleWatch,
    AirPods,
    SmartTV,
    IoTDevice,
    NetworkDevice,
    Printer,
    GameConsole,
}

impl Default for DetectedDeviceType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Copy, Default, Encode, Decode)]
pub struct Position {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct ScannedDevice {
    pub mac_hash: H256,
    pub rssi: i8,
    pub signal_type: ScanSignalType,
    pub device_type: DetectedDeviceType,
    pub vendor: Option<[u8; 32]>,
    pub device_name: Option<[u8; 64]>,
    pub frequency: Option<u16>,
    pub detected_at: u64,
}

#[derive(Debug, Clone)]
pub struct ScanResults {
    pub devices: Vec<ScannedDevice>,
    pub last_scan: Option<SystemTime>,
    pub scan_sequence: u64,
    pub last_emitted_sequence: Option<u64>,
}

impl Default for ScanResults {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            last_scan: None,
            scan_sequence: 0,
            last_emitted_sequence: None,
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct DeviceScanInherentData {
    pub devices: Vec<ScannedDevice>,
    pub reporter_position: Position,
    pub scan_timestamp: u64,
}
