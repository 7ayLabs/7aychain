#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;
pub mod weights;

#[cfg(test)]
mod tests;

use alloc::vec::Vec;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_inherents::{InherentData, InherentIdentifier, IsFatalError};
use sp_runtime::Saturating;

pub type MaxDevicesPerScan = ConstU32<100>;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"devscan0";
pub const MAX_DEVICES_PER_INHERENT: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Default)]
pub struct Position {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub struct ScannedDevice {
    pub mac_hash: H256,
    pub rssi: i8,
    pub signal_type: ScanSignalType,
    pub device_type: DetectedDeviceType,
    pub vendor_hash: Option<H256>,
    pub name_hash: Option<H256>,
    pub frequency: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo)]
pub struct DeviceScanInherentData {
    pub devices: Vec<ScannedDevice>,
    pub reporter_position: Position,
    pub scan_timestamp: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct TrackedScannedDevice<BlockNumber> {
    pub mac_hash: H256,
    pub device_type: DetectedDeviceType,
    pub signal_type: ScanSignalType,
    pub last_rssi: i8,
    pub first_seen: BlockNumber,
    pub last_seen: BlockNumber,
    pub detection_count: u32,
    pub last_position: Position,
}

/// Reason for device eviction from tracking
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub enum EvictionReason {
    /// Device evicted due to capacity limits (LRU - least recently used)
    CapacityLRU,
}

#[derive(Encode, sp_runtime::RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode))]
pub enum InherentError {
    TooManyDevices,
    InvalidTimestamp,
}

impl IsFatalError for InherentError {
    fn is_fatal_error(&self) -> bool {
        false
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    pub use crate::weights::WeightInfo;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;

        #[pallet::constant]
        type MaxTrackedDevices: Get<u32>;

        #[pallet::constant]
        type DeviceStaleBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MaxHistoryPerDevice: Get<u32>;
    }

    #[pallet::storage]
    #[pallet::getter(fn device_count)]
    pub type DeviceCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn tracked_devices)]
    pub type TrackedDevices<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, TrackedScannedDevice<BlockNumberFor<T>>>;

    #[pallet::storage]
    #[pallet::getter(fn active_device_count)]
    pub type ActiveDeviceCount<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn last_scan_timestamp)]
    pub type LastScanTimestamp<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn device_type_count)]
    pub type DeviceTypeCount<T> =
        StorageMap<_, Blake2_128Concat, DetectedDeviceType, u32, ValueQuery>;

    #[pallet::storage]
    pub type ScanDataReceived<T> = StorageValue<_, bool, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: core::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            DeviceCount::<T>::put(0u64);
            ActiveDeviceCount::<T>::put(0u32);
            LastScanTimestamp::<T>::put(0u64);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            ScanDataReceived::<T>::put(false);
            Weight::from_parts(1_000, 0)
        }

        fn on_finalize(n: BlockNumberFor<T>) {
            if (n % 100u32.into()).is_zero() {
                Self::cleanup_stale_devices(n);
            }
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        NewDeviceDetected {
            mac_hash: H256,
            device_type: DetectedDeviceType,
            signal_type: ScanSignalType,
        },
        DeviceUpdated {
            mac_hash: H256,
            rssi: i8,
            detection_count: u32,
        },
        DeviceStale {
            mac_hash: H256,
            last_seen: BlockNumberFor<T>,
        },
        /// Device was evicted due to capacity limits
        DeviceEvicted {
            mac_hash: H256,
            reason: EvictionReason,
        },
        ScanProcessed {
            device_count: u32,
            timestamp: u64,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        TooManyDevices,
        InvalidInherentData,
        ScanDataAlreadyReceived,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::process_scan_data(data.devices.len() as u32))]
        pub fn set_scan_data(origin: OriginFor<T>, data: DeviceScanInherentData) -> DispatchResult {
            ensure_none(origin)?;
            ensure!(!ScanDataReceived::<T>::get(), Error::<T>::ScanDataAlreadyReceived);
            ensure!(
                data.devices.len() <= MAX_DEVICES_PER_INHERENT as usize,
                Error::<T>::TooManyDevices
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            let mut new_count = 0u32;
            let mut updated_count = 0u32;

            for device in data.devices.iter() {
                let is_new = !TrackedDevices::<T>::contains_key(device.mac_hash);

                if is_new {
                    // Enforce MaxTrackedDevices with LRU eviction
                    let current_count = ActiveDeviceCount::<T>::get();
                    if current_count >= T::MaxTrackedDevices::get() {
                        // Find and evict the least recently used device
                        if let Some((oldest_hash, oldest_device)) = TrackedDevices::<T>::iter()
                            .min_by_key(|(_, d)| d.last_seen)
                        {
                            // Remove the oldest device
                            TrackedDevices::<T>::remove(oldest_hash);
                            DeviceTypeCount::<T>::mutate(oldest_device.device_type, |c| *c = c.saturating_sub(1));
                            ActiveDeviceCount::<T>::mutate(|c| *c = c.saturating_sub(1));

                            Self::deposit_event(Event::DeviceEvicted {
                                mac_hash: oldest_hash,
                                reason: EvictionReason::CapacityLRU,
                            });
                        }
                    }

                    let tracked = TrackedScannedDevice {
                        mac_hash: device.mac_hash,
                        device_type: device.device_type,
                        signal_type: device.signal_type,
                        last_rssi: device.rssi,
                        first_seen: block_number,
                        last_seen: block_number,
                        detection_count: 1,
                        last_position: data.reporter_position,
                    };

                    TrackedDevices::<T>::insert(device.mac_hash, tracked);
                    DeviceCount::<T>::mutate(|c| *c = c.saturating_add(1));
                    ActiveDeviceCount::<T>::mutate(|c| *c = c.saturating_add(1));
                    DeviceTypeCount::<T>::mutate(device.device_type, |c| *c = c.saturating_add(1));

                    Self::deposit_event(Event::NewDeviceDetected {
                        mac_hash: device.mac_hash,
                        device_type: device.device_type,
                        signal_type: device.signal_type,
                    });

                    new_count += 1;
                } else {
                    TrackedDevices::<T>::mutate(device.mac_hash, |d| {
                        if let Some(dev) = d {
                            dev.last_rssi = device.rssi;
                            dev.last_seen = block_number;
                            dev.detection_count = dev.detection_count.saturating_add(1);
                            dev.last_position = data.reporter_position;

                            Self::deposit_event(Event::DeviceUpdated {
                                mac_hash: device.mac_hash,
                                rssi: device.rssi,
                                detection_count: dev.detection_count,
                            });
                        }
                    });

                    updated_count += 1;
                }
            }

            LastScanTimestamp::<T>::put(data.scan_timestamp);
            ScanDataReceived::<T>::put(true);

            Self::deposit_event(Event::ScanProcessed {
                device_count: new_count + updated_count,
                timestamp: data.scan_timestamp,
            });

            Ok(())
        }
    }

    #[pallet::inherent]
    impl<T: Config> ProvideInherent for Pallet<T> {
        type Call = Call<T>;
        type Error = InherentError;
        const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

        fn create_inherent(data: &InherentData) -> Option<Self::Call> {
            let scan_data = data
                .get_data::<DeviceScanInherentData>(&INHERENT_IDENTIFIER)
                .ok()
                .flatten()?;

            if scan_data.devices.is_empty() {
                return None;
            }

            Some(Call::set_scan_data { data: scan_data })
        }

        fn check_inherent(call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
            let Call::set_scan_data { data: call_data } = call else {
                return Ok(());
            };

            if call_data.devices.len() > MAX_DEVICES_PER_INHERENT as usize {
                return Err(InherentError::TooManyDevices);
            }

            Ok(())
        }

        fn is_inherent(call: &Self::Call) -> bool {
            matches!(call, Call::set_scan_data { .. })
        }
    }

    impl<T: Config> Pallet<T> {
        fn cleanup_stale_devices(current_block: BlockNumberFor<T>) {
            let stale_threshold = T::DeviceStaleBlocks::get();

            for (mac_hash, device) in TrackedDevices::<T>::iter() {
                let blocks_since = current_block.saturating_sub(device.last_seen);

                if blocks_since >= stale_threshold {
                    Self::deposit_event(Event::DeviceStale {
                        mac_hash,
                        last_seen: device.last_seen,
                    });

                    ActiveDeviceCount::<T>::mutate(|c| *c = c.saturating_sub(1));
                }
            }
        }

        pub fn get_active_devices() -> Vec<TrackedScannedDevice<BlockNumberFor<T>>> {
            let current_block = frame_system::Pallet::<T>::block_number();
            let stale_threshold = T::DeviceStaleBlocks::get();

            TrackedDevices::<T>::iter()
                .filter(|(_, d)| current_block.saturating_sub(d.last_seen) < stale_threshold)
                .map(|(_, d)| d)
                .collect()
        }

        pub fn get_devices_by_type(device_type: DetectedDeviceType) -> Vec<H256> {
            TrackedDevices::<T>::iter()
                .filter(|(_, d)| d.device_type == device_type)
                .map(|(hash, _)| hash)
                .collect()
        }

        pub fn get_statistics() -> (u64, u32, u64) {
            (
                DeviceCount::<T>::get(),
                ActiveDeviceCount::<T>::get(),
                LastScanTimestamp::<T>::get(),
            )
        }
    }
}
