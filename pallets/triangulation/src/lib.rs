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
use sp_runtime::Saturating;

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Default,
    Hash,
)]
pub struct ReporterId(pub u64);

impl ReporterId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum SignalType {
    Wifi,
    Bluetooth,
    Ble,
    Zigbee,
    Unknown,
}

impl Default for SignalType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum DeviceState {
    Active,
    LowPower,
    Sleeping,
    Shielded,
    TurnedOff,
    Suspicious,
    Lost,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Default,
)]
pub struct Position {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct SignalReading<BlockNumber> {
    pub reporter_id: ReporterId,
    pub rssi: i8,
    pub signal_type: SignalType,
    pub frequency: u16,
    pub recorded_at: BlockNumber,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct TrackedDevice<BlockNumber> {
    pub mac_hash: H256,
    pub signal_type: SignalType,
    pub state: DeviceState,
    pub estimated_position: Position,
    pub confidence: u8,
    pub first_seen: BlockNumber,
    pub last_seen: BlockNumber,
    pub reading_count: u32,
    pub consecutive_misses: u32,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct Reporter<BlockNumber> {
    pub id: ReporterId,
    pub position: Position,
    pub registered_at: BlockNumber,
    pub active: bool,
    pub reading_count: u64,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct SignalHistoryEntry<BlockNumber> {
    pub reading: SignalReading<BlockNumber>,
    pub position_at_time: Position,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct GhostEvent<BlockNumber> {
    pub mac_hash: H256,
    pub last_position: Position,
    pub last_seen: BlockNumber,
    pub disappeared_at: BlockNumber,
    pub previous_state: DeviceState,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    pub use crate::weights::WeightInfo;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
        type WeightInfo: WeightInfo;

        #[pallet::constant]
        type MaxReporters: Get<u32>;

        #[pallet::constant]
        type MaxReadingsPerDevice: Get<u32>;

        #[pallet::constant]
        type MaxHistoryEntries: Get<u32>;

        #[pallet::constant]
        type InactiveTimeoutBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type LostTimeoutBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MinReadingsForActive: Get<u32>;

        #[pallet::constant]
        type SignalRetentionBlocks: Get<BlockNumberFor<Self>>;
    }

    #[pallet::storage]
    #[pallet::getter(fn reporter_count)]
    pub type ReporterCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn reporters)]
    pub type Reporters<T: Config> = StorageMap<_, Blake2_128Concat, ReporterId, Reporter<BlockNumberFor<T>>>;

    #[pallet::storage]
    #[pallet::getter(fn tracked_devices)]
    pub type TrackedDevices<T: Config> = StorageMap<_, Blake2_128Concat, H256, TrackedDevice<BlockNumberFor<T>>>;

    #[pallet::storage]
    #[pallet::getter(fn device_count)]
    pub type DeviceCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn signal_history)]
    pub type SignalHistory<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        H256,
        Blake2_128Concat,
        BlockNumberFor<T>,
        SignalHistoryEntry<BlockNumberFor<T>>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn ghost_events)]
    pub type GhostEvents<T: Config> = StorageMap<_, Blake2_128Concat, H256, GhostEvent<BlockNumberFor<T>>>;

    #[pallet::storage]
    #[pallet::getter(fn active_device_count)]
    pub type ActiveDeviceCount<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn ghost_count)]
    pub type GhostCount<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            ReporterCount::<T>::put(0u64);
            DeviceCount::<T>::put(0u64);
            ActiveDeviceCount::<T>::put(0u32);
            GhostCount::<T>::put(0u32);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            Self::detect_ghosts(block_number);
            Self::cleanup_old_history(block_number);
            Weight::from_parts(50_000, 0)
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ReporterRegistered {
            reporter_id: ReporterId,
            position: Position,
        },
        ReporterDeregistered {
            reporter_id: ReporterId,
        },
        SignalDetected {
            mac_hash: H256,
            reporter_id: ReporterId,
            rssi: i8,
            signal_type: SignalType,
        },
        DeviceStateChanged {
            mac_hash: H256,
            old_state: DeviceState,
            new_state: DeviceState,
        },
        GhostDetected {
            mac_hash: H256,
            last_position: Position,
            last_seen: BlockNumberFor<T>,
        },
        DeviceRecovered {
            mac_hash: H256,
            new_position: Position,
        },
        PositionUpdated {
            mac_hash: H256,
            position: Position,
            confidence: u8,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        ReporterNotFound,
        ReporterAlreadyExists,
        MaxReportersReached,
        DeviceNotFound,
        ReporterNotActive,
        InvalidRssi,
        MaxReadingsReached,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_reporter())]
        pub fn register_reporter(
            origin: OriginFor<T>,
            position: Position,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let count = ReporterCount::<T>::get();
            ensure!(count < T::MaxReporters::get() as u64, Error::<T>::MaxReportersReached);

            let reporter_id = ReporterId::new(count);
            let block_number = frame_system::Pallet::<T>::block_number();

            let reporter = Reporter {
                id: reporter_id,
                position: position.clone(),
                registered_at: block_number,
                active: true,
                reading_count: 0,
            };

            Reporters::<T>::insert(reporter_id, reporter);
            ReporterCount::<T>::put(count.saturating_add(1));

            Self::deposit_event(Event::ReporterRegistered {
                reporter_id,
                position,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::deregister_reporter())]
        pub fn deregister_reporter(
            origin: OriginFor<T>,
            reporter_id: ReporterId,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            Reporters::<T>::try_mutate(reporter_id, |reporter| -> DispatchResult {
                let r = reporter.as_mut().ok_or(Error::<T>::ReporterNotFound)?;
                r.active = false;

                Self::deposit_event(Event::ReporterDeregistered { reporter_id });

                Ok(())
            })
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::report_signal())]
        pub fn report_signal(
            origin: OriginFor<T>,
            reporter_id: ReporterId,
            mac_hash: H256,
            rssi: i8,
            signal_type: SignalType,
            frequency: u16,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            ensure!(rssi >= -120 && rssi <= 0, Error::<T>::InvalidRssi);

            let reporter = Reporters::<T>::get(reporter_id).ok_or(Error::<T>::ReporterNotFound)?;
            ensure!(reporter.active, Error::<T>::ReporterNotActive);

            let block_number = frame_system::Pallet::<T>::block_number();

            let reading = SignalReading {
                reporter_id,
                rssi,
                signal_type,
                frequency,
                recorded_at: block_number,
            };

            Reporters::<T>::mutate(reporter_id, |r| {
                if let Some(rep) = r {
                    rep.reading_count = rep.reading_count.saturating_add(1);
                }
            });

            let is_new_device = !TrackedDevices::<T>::contains_key(mac_hash);

            if is_new_device {
                let device = TrackedDevice {
                    mac_hash,
                    signal_type,
                    state: DeviceState::Active,
                    estimated_position: reporter.position.clone(),
                    confidence: 30,
                    first_seen: block_number,
                    last_seen: block_number,
                    reading_count: 1,
                    consecutive_misses: 0,
                };

                TrackedDevices::<T>::insert(mac_hash, device);
                DeviceCount::<T>::mutate(|c| *c = c.saturating_add(1));
                ActiveDeviceCount::<T>::mutate(|c| *c = c.saturating_add(1));
            } else {
                TrackedDevices::<T>::mutate(mac_hash, |device| {
                    if let Some(d) = device {
                        let old_state = d.state;

                        d.last_seen = block_number;
                        d.reading_count = d.reading_count.saturating_add(1);
                        d.consecutive_misses = 0;

                        let new_position = Self::calculate_position(&reporter.position, &d.estimated_position, rssi);
                        d.estimated_position = new_position.clone();

                        d.confidence = d.confidence.saturating_add(5).min(100);

                        if d.reading_count >= T::MinReadingsForActive::get() {
                            d.state = DeviceState::Active;
                        }

                        if old_state != d.state {
                            Self::deposit_event(Event::DeviceStateChanged {
                                mac_hash,
                                old_state,
                                new_state: d.state,
                            });
                        }

                        if matches!(old_state, DeviceState::Lost | DeviceState::Shielded | DeviceState::TurnedOff) {
                            GhostEvents::<T>::remove(mac_hash);
                            GhostCount::<T>::mutate(|c| *c = c.saturating_sub(1));

                            Self::deposit_event(Event::DeviceRecovered {
                                mac_hash,
                                new_position,
                            });
                        }
                    }
                });
            }

            let history_entry = SignalHistoryEntry {
                reading,
                position_at_time: reporter.position.clone(),
            };

            SignalHistory::<T>::insert(mac_hash, block_number, history_entry);

            Self::deposit_event(Event::SignalDetected {
                mac_hash,
                reporter_id,
                rssi,
                signal_type,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::update_reporter_position())]
        pub fn update_reporter_position(
            origin: OriginFor<T>,
            reporter_id: ReporterId,
            new_position: Position,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            Reporters::<T>::try_mutate(reporter_id, |reporter| -> DispatchResult {
                let r = reporter.as_mut().ok_or(Error::<T>::ReporterNotFound)?;
                r.position = new_position;
                Ok(())
            })
        }
    }

    impl<T: Config> Pallet<T> {
        fn calculate_position(reporter_pos: &Position, current_pos: &Position, rssi: i8) -> Position {
            let weight = ((rssi + 120) as i64).max(1);
            let total_weight = weight + 100;

            Position {
                x: (reporter_pos.x * weight + current_pos.x * 100) / total_weight,
                y: (reporter_pos.y * weight + current_pos.y * 100) / total_weight,
                z: (reporter_pos.z * weight + current_pos.z * 100) / total_weight,
            }
        }

        fn detect_ghosts(current_block: BlockNumberFor<T>) {
            let inactive_timeout = T::InactiveTimeoutBlocks::get();
            let lost_timeout = T::LostTimeoutBlocks::get();

            for (mac_hash, mut device) in TrackedDevices::<T>::iter() {
                let blocks_since = current_block.saturating_sub(device.last_seen);
                let old_state = device.state;

                if blocks_since >= lost_timeout {
                    if !matches!(device.state, DeviceState::Lost) {
                        device.state = DeviceState::Lost;
                        device.consecutive_misses = device.consecutive_misses.saturating_add(1);

                        let ghost = GhostEvent {
                            mac_hash,
                            last_position: device.estimated_position.clone(),
                            last_seen: device.last_seen,
                            disappeared_at: current_block,
                            previous_state: old_state,
                        };

                        GhostEvents::<T>::insert(mac_hash, ghost);
                        GhostCount::<T>::mutate(|c| *c = c.saturating_add(1));

                        Self::deposit_event(Event::GhostDetected {
                            mac_hash,
                            last_position: device.estimated_position.clone(),
                            last_seen: device.last_seen,
                        });

                        Self::deposit_event(Event::DeviceStateChanged {
                            mac_hash,
                            old_state,
                            new_state: DeviceState::Lost,
                        });

                        TrackedDevices::<T>::insert(mac_hash, device);
                    }
                } else if blocks_since >= inactive_timeout {
                    if matches!(device.state, DeviceState::Active) {
                        device.consecutive_misses = device.consecutive_misses.saturating_add(1);

                        device.state = if device.consecutive_misses >= 3 {
                            DeviceState::Shielded
                        } else {
                            DeviceState::Sleeping
                        };

                        device.confidence = device.confidence.saturating_sub(10);

                        Self::deposit_event(Event::DeviceStateChanged {
                            mac_hash,
                            old_state,
                            new_state: device.state,
                        });

                        TrackedDevices::<T>::insert(mac_hash, device);
                    }
                }
            }
        }

        fn cleanup_old_history(_current_block: BlockNumberFor<T>) {
        }

        pub fn get_device_history(mac_hash: H256) -> Vec<(BlockNumberFor<T>, SignalHistoryEntry<BlockNumberFor<T>>)> {
            SignalHistory::<T>::iter_prefix(mac_hash).collect()
        }

        pub fn get_last_known_position(mac_hash: H256) -> Option<Position> {
            TrackedDevices::<T>::get(mac_hash).map(|d| d.estimated_position)
        }

        pub fn get_device_state(mac_hash: H256) -> Option<DeviceState> {
            TrackedDevices::<T>::get(mac_hash).map(|d| d.state)
        }

        pub fn is_ghost(mac_hash: H256) -> bool {
            GhostEvents::<T>::contains_key(mac_hash)
        }

        pub fn get_ghost_info(mac_hash: H256) -> Option<GhostEvent<BlockNumberFor<T>>> {
            GhostEvents::<T>::get(mac_hash)
        }
    }
}
