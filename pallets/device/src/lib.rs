#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub mod weights;

#[cfg(test)]
mod tests;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use seveny_primitives::types::ActorId;
use sp_core::H256;
use sp_runtime::Saturating;
use sp_std::vec::Vec;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Default, Hash,
)]
pub struct DeviceId(pub u64);

impl DeviceId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum DeviceType {
    Mobile,
    Desktop,
    Server,
    IoT,
    Hardware,
    Virtual,
}

impl Default for DeviceType {
    fn default() -> Self {
        Self::Mobile
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum DeviceStatus {
    Pending,
    Active,
    Suspended,
    Revoked,
    Compromised,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum AttestationType {
    SelfSigned,
    TrustedParty,
    HardwareBacked,
    Tpm,
    SecureEnclave,
}

impl Default for AttestationType {
    fn default() -> Self {
        Self::SelfSigned
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Device<T: Config> {
    pub id: DeviceId,
    pub owner: ActorId,
    pub device_type: DeviceType,
    pub public_key_hash: H256,
    pub attestation_type: AttestationType,
    pub status: DeviceStatus,
    pub registered_at: BlockNumberFor<T>,
    pub last_active: BlockNumberFor<T>,
    pub trust_score: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct DeviceAttestation<T: Config> {
    pub device: DeviceId,
    pub attestation_hash: H256,
    pub attester: Option<ActorId>,
    pub attested_at: BlockNumberFor<T>,
    pub valid_until: Option<BlockNumberFor<T>>,
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
        type MaxDevicesPerActor: Get<u32>;

        #[pallet::constant]
        type AttestationValidityBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type InitialTrustScore: Get<u8>;
    }

    #[pallet::storage]
    #[pallet::getter(fn device_count)]
    pub type DeviceCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn devices)]
    pub type Devices<T: Config> = StorageMap<_, Blake2_128Concat, DeviceId, Device<T>>;

    #[pallet::storage]
    #[pallet::getter(fn actor_devices)]
    pub type ActorDevices<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, ActorId, Blake2_128Concat, DeviceId, ()>;

    #[pallet::storage]
    #[pallet::getter(fn device_count_per_actor)]
    pub type DeviceCountPerActor<T: Config> =
        StorageMap<_, Blake2_128Concat, ActorId, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn attestations)]
    pub type Attestations<T: Config> =
        StorageMap<_, Blake2_128Concat, DeviceId, DeviceAttestation<T>>;

    #[pallet::storage]
    #[pallet::getter(fn public_key_device)]
    pub type PublicKeyDevice<T: Config> = StorageMap<_, Blake2_128Concat, H256, DeviceId>;

    #[pallet::storage]
    #[pallet::getter(fn active_device_count)]
    pub type ActiveDeviceCount<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            DeviceCount::<T>::put(0u64);
            ActiveDeviceCount::<T>::put(0u32);
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        DeviceRegistered {
            device_id: DeviceId,
            owner: ActorId,
            device_type: DeviceType,
        },
        DeviceActivated {
            device_id: DeviceId,
        },
        DeviceSuspended {
            device_id: DeviceId,
            reason: H256,
        },
        DeviceRevoked {
            device_id: DeviceId,
        },
        DeviceMarkedCompromised {
            device_id: DeviceId,
        },
        AttestationSubmitted {
            device_id: DeviceId,
            attestation_hash: H256,
        },
        TrustScoreUpdated {
            device_id: DeviceId,
            old_score: u8,
            new_score: u8,
        },
        DeviceActivityRecorded {
            device_id: DeviceId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        DeviceNotFound,
        MaxDevicesReached,
        DeviceAlreadyExists,
        NotDeviceOwner,
        DeviceNotActive,
        DeviceAlreadyActive,
        InvalidPublicKey,
        PublicKeyAlreadyUsed,
        AttestationExpired,
        InvalidAttestation,
        DeviceCompromised,
        CannotReactivateRevokedDevice,
        InvalidTrustScore,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_device())]
        pub fn register_device(
            origin: OriginFor<T>,
            owner: ActorId,
            device_type: DeviceType,
            public_key_hash: H256,
            attestation_type: AttestationType,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            ensure!(
                !PublicKeyDevice::<T>::contains_key(public_key_hash),
                Error::<T>::PublicKeyAlreadyUsed
            );

            let device_count = DeviceCountPerActor::<T>::get(owner);
            ensure!(
                device_count < T::MaxDevicesPerActor::get(),
                Error::<T>::MaxDevicesReached
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            let device_id = Self::next_device_id();

            let device = Device {
                id: device_id,
                owner,
                device_type,
                public_key_hash,
                attestation_type,
                status: DeviceStatus::Pending,
                registered_at: block_number,
                last_active: block_number,
                trust_score: T::InitialTrustScore::get(),
            };

            Devices::<T>::insert(device_id, device);
            ActorDevices::<T>::insert(owner, device_id, ());
            DeviceCountPerActor::<T>::mutate(owner, |count| *count = count.saturating_add(1));
            PublicKeyDevice::<T>::insert(public_key_hash, device_id);

            Self::deposit_event(Event::DeviceRegistered {
                device_id,
                owner,
                device_type,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::activate_device())]
        pub fn activate_device(origin: OriginFor<T>, device_id: DeviceId) -> DispatchResult {
            ensure_signed(origin)?;

            Devices::<T>::try_mutate(device_id, |device| -> DispatchResult {
                let d = device.as_mut().ok_or(Error::<T>::DeviceNotFound)?;

                ensure!(
                    d.status == DeviceStatus::Pending,
                    Error::<T>::DeviceAlreadyActive
                );

                d.status = DeviceStatus::Active;

                ActiveDeviceCount::<T>::mutate(|count| *count = count.saturating_add(1));

                Self::deposit_event(Event::DeviceActivated { device_id });

                Ok(())
            })
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::suspend_device())]
        pub fn suspend_device(
            origin: OriginFor<T>,
            device_id: DeviceId,
            reason: H256,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            Devices::<T>::try_mutate(device_id, |device| -> DispatchResult {
                let d = device.as_mut().ok_or(Error::<T>::DeviceNotFound)?;

                ensure!(
                    d.status == DeviceStatus::Active,
                    Error::<T>::DeviceNotActive
                );

                d.status = DeviceStatus::Suspended;

                ActiveDeviceCount::<T>::mutate(|count| *count = count.saturating_sub(1));

                Self::deposit_event(Event::DeviceSuspended { device_id, reason });

                Ok(())
            })
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::revoke_device())]
        pub fn revoke_device(origin: OriginFor<T>, device_id: DeviceId) -> DispatchResult {
            ensure_signed(origin)?;

            Devices::<T>::try_mutate(device_id, |device| -> DispatchResult {
                let d = device.as_mut().ok_or(Error::<T>::DeviceNotFound)?;

                if d.status == DeviceStatus::Active {
                    ActiveDeviceCount::<T>::mutate(|count| *count = count.saturating_sub(1));
                }

                d.status = DeviceStatus::Revoked;

                Self::deposit_event(Event::DeviceRevoked { device_id });

                Ok(())
            })
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::mark_compromised())]
        pub fn mark_compromised(origin: OriginFor<T>, device_id: DeviceId) -> DispatchResult {
            ensure_root(origin)?;

            Devices::<T>::try_mutate(device_id, |device| -> DispatchResult {
                let d = device.as_mut().ok_or(Error::<T>::DeviceNotFound)?;

                if d.status == DeviceStatus::Active {
                    ActiveDeviceCount::<T>::mutate(|count| *count = count.saturating_sub(1));
                }

                d.status = DeviceStatus::Compromised;

                Self::deposit_event(Event::DeviceMarkedCompromised { device_id });

                Ok(())
            })
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::submit_attestation())]
        pub fn submit_attestation(
            origin: OriginFor<T>,
            device_id: DeviceId,
            attestation_hash: H256,
            attester: Option<ActorId>,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            ensure!(
                Devices::<T>::contains_key(device_id),
                Error::<T>::DeviceNotFound
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            let valid_until = Some(block_number.saturating_add(T::AttestationValidityBlocks::get()));

            let attestation = DeviceAttestation {
                device: device_id,
                attestation_hash,
                attester,
                attested_at: block_number,
                valid_until,
            };

            Attestations::<T>::insert(device_id, attestation);

            Self::deposit_event(Event::AttestationSubmitted {
                device_id,
                attestation_hash,
            });

            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::update_trust_score())]
        pub fn update_trust_score(
            origin: OriginFor<T>,
            device_id: DeviceId,
            new_score: u8,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(new_score <= 100, Error::<T>::InvalidTrustScore);

            Devices::<T>::try_mutate(device_id, |device| -> DispatchResult {
                let d = device.as_mut().ok_or(Error::<T>::DeviceNotFound)?;
                let old_score = d.trust_score;
                d.trust_score = new_score;

                Self::deposit_event(Event::TrustScoreUpdated {
                    device_id,
                    old_score,
                    new_score,
                });

                Ok(())
            })
        }

        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::record_activity())]
        pub fn record_activity(origin: OriginFor<T>, device_id: DeviceId) -> DispatchResult {
            ensure_signed(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();

            Devices::<T>::try_mutate(device_id, |device| -> DispatchResult {
                let d = device.as_mut().ok_or(Error::<T>::DeviceNotFound)?;

                ensure!(
                    d.status == DeviceStatus::Active,
                    Error::<T>::DeviceNotActive
                );

                d.last_active = block_number;

                Self::deposit_event(Event::DeviceActivityRecorded { device_id });

                Ok(())
            })
        }

        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::reactivate_device())]
        pub fn reactivate_device(origin: OriginFor<T>, device_id: DeviceId) -> DispatchResult {
            ensure_signed(origin)?;

            Devices::<T>::try_mutate(device_id, |device| -> DispatchResult {
                let d = device.as_mut().ok_or(Error::<T>::DeviceNotFound)?;

                ensure!(
                    d.status == DeviceStatus::Suspended,
                    Error::<T>::CannotReactivateRevokedDevice
                );

                d.status = DeviceStatus::Active;

                ActiveDeviceCount::<T>::mutate(|count| *count = count.saturating_add(1));

                Self::deposit_event(Event::DeviceActivated { device_id });

                Ok(())
            })
        }
    }

    impl<T: Config> Pallet<T> {
        fn next_device_id() -> DeviceId {
            let id = DeviceCount::<T>::get();
            DeviceCount::<T>::put(id.saturating_add(1));
            DeviceId::new(id)
        }

        pub fn get_actor_devices(actor: ActorId) -> Vec<DeviceId> {
            ActorDevices::<T>::iter_prefix(actor)
                .map(|(device_id, _)| device_id)
                .collect()
        }

        pub fn get_active_devices(actor: ActorId) -> Vec<DeviceId> {
            ActorDevices::<T>::iter_prefix(actor)
                .filter_map(|(device_id, _)| {
                    Devices::<T>::get(device_id)
                        .filter(|d| d.status == DeviceStatus::Active)
                        .map(|_| device_id)
                })
                .collect()
        }

        pub fn is_device_active(device_id: DeviceId) -> bool {
            Devices::<T>::get(device_id)
                .is_some_and(|d| d.status == DeviceStatus::Active)
        }

        pub fn get_device_trust_score(device_id: DeviceId) -> u8 {
            Devices::<T>::get(device_id)
                .map(|d| d.trust_score)
                .unwrap_or(0)
        }

        pub fn is_attestation_valid(device_id: DeviceId, block_number: BlockNumberFor<T>) -> bool {
            Attestations::<T>::get(device_id)
                .is_some_and(|a| a.valid_until.map_or(true, |until| block_number <= until))
        }

        pub fn get_total_active_devices() -> u32 {
            ActiveDeviceCount::<T>::get()
        }
    }
}
