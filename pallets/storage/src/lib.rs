#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub mod weights;

#[cfg(test)]
mod tests;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use seveny_primitives::types::{ActorId, EpochId};
use sp_core::H256;
use sp_runtime::Saturating;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Default, Hash)]
pub struct DataKey(pub H256);

impl DataKey {
    pub fn new(key: H256) -> Self {
        Self(key)
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let hash = sp_core::blake2_256(bytes);
        Self(H256(hash))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum DataType {
    Presence,
    Commitment,
    Proof,
    Metadata,
    Temporary,
}

impl Default for DataType {
    fn default() -> Self {
        Self::Temporary
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum StorageStatus {
    Active,
    Expired,
    Deleted,
}

impl Default for StorageStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum RetentionPolicy {
    EpochBound,
    TimeBound,
    Persistent,
    OneTime,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::EpochBound
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct EphemeralEntry<T: Config> {
    pub key: DataKey,
    pub data_hash: H256,
    pub data_type: DataType,
    pub owner: ActorId,
    pub epoch: EpochId,
    pub status: StorageStatus,
    pub retention: RetentionPolicy,
    pub created_at: BlockNumberFor<T>,
    pub expires_at: Option<BlockNumberFor<T>>,
    pub size_bytes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct StorageQuota<T: Config> {
    pub actor: ActorId,
    pub max_entries: u32,
    pub max_bytes: u64,
    pub used_entries: u32,
    pub used_bytes: u64,
    pub last_updated: BlockNumberFor<T>,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct EpochStorage<T: Config> {
    pub epoch: EpochId,
    pub entry_count: u32,
    pub total_bytes: u64,
    pub created_at: BlockNumberFor<T>,
    pub finalized: bool,
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
        type MaxDataSize: Get<u32>;

        #[pallet::constant]
        type MaxEntriesPerActor: Get<u32>;

        #[pallet::constant]
        type MaxEntriesPerEpoch: Get<u32>;

        #[pallet::constant]
        type DefaultRetentionBlocks: Get<BlockNumberFor<Self>>;
    }

    #[pallet::storage]
    #[pallet::getter(fn entry_count)]
    pub type EntryCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn ephemeral_data)]
    pub type EphemeralData<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Blake2_128Concat, EpochId>,
            NMapKey<Blake2_128Concat, ActorId>,
            NMapKey<Blake2_128Concat, DataKey>,
        ),
        EphemeralEntry<T>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn actor_entries)]
    pub type ActorEntries<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, ActorId, Blake2_128Concat, DataKey, ()>;

    #[pallet::storage]
    #[pallet::getter(fn epoch_entries)]
    pub type EpochEntries<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, EpochId, Blake2_128Concat, DataKey, ()>;

    #[pallet::storage]
    #[pallet::getter(fn storage_quotas)]
    pub type StorageQuotas<T: Config> = StorageMap<_, Blake2_128Concat, ActorId, StorageQuota<T>>;

    #[pallet::storage]
    #[pallet::getter(fn epoch_storage)]
    pub type EpochStorageInfo<T: Config> = StorageMap<_, Blake2_128Concat, EpochId, EpochStorage<T>>;

    #[pallet::storage]
    #[pallet::getter(fn active_entries)]
    pub type ActiveEntries<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn total_storage_bytes)]
    pub type TotalStorageBytes<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            EntryCount::<T>::put(0u64);
            ActiveEntries::<T>::put(0u64);
            TotalStorageBytes::<T>::put(0u64);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            Self::cleanup_expired_entries(n)
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        DataStored {
            epoch: EpochId,
            actor: ActorId,
            key: DataKey,
            data_type: DataType,
            size: u32,
        },
        DataUpdated {
            epoch: EpochId,
            actor: ActorId,
            key: DataKey,
        },
        DataDeleted {
            epoch: EpochId,
            actor: ActorId,
            key: DataKey,
        },
        DataExpired {
            epoch: EpochId,
            actor: ActorId,
            key: DataKey,
        },
        EpochFinalized {
            epoch: EpochId,
            entries: u32,
        },
        QuotaUpdated {
            actor: ActorId,
            max_entries: u32,
            max_bytes: u64,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        DataNotFound,
        DataAlreadyExists,
        DataTooLarge,
        QuotaExceeded,
        EpochQuotaExceeded,
        NotDataOwner,
        DataExpired,
        InvalidDataType,
        InvalidRetentionPolicy,
        EpochNotActive,
        CannotModifyFinalizedData,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::store_data())]
        pub fn store_data(
            origin: OriginFor<T>,
            epoch: EpochId,
            key: DataKey,
            data_hash: H256,
            data_type: DataType,
            size_bytes: u32,
            retention: RetentionPolicy,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                size_bytes <= T::MaxDataSize::get(),
                Error::<T>::DataTooLarge
            );

            let actor = Self::account_to_actor(who);

            ensure!(
                !EphemeralData::<T>::contains_key((epoch, actor, key)),
                Error::<T>::DataAlreadyExists
            );

            Self::check_actor_quota(actor, size_bytes)?;
            Self::check_epoch_quota(epoch)?;

            let block_number = frame_system::Pallet::<T>::block_number();
            let expires_at = match retention {
                RetentionPolicy::TimeBound => {
                    Some(block_number.saturating_add(T::DefaultRetentionBlocks::get()))
                }
                RetentionPolicy::OneTime => Some(block_number.saturating_add(1u32.into())),
                _ => None,
            };

            let entry = EphemeralEntry {
                key,
                data_hash,
                data_type,
                owner: actor,
                epoch,
                status: StorageStatus::Active,
                retention,
                created_at: block_number,
                expires_at,
                size_bytes,
            };

            EphemeralData::<T>::insert((epoch, actor, key), entry);
            ActorEntries::<T>::insert(actor, key, ());
            EpochEntries::<T>::insert(epoch, key, ());

            Self::update_actor_quota(actor, size_bytes, true);
            Self::update_epoch_storage(epoch, size_bytes, true);

            EntryCount::<T>::mutate(|c| *c = c.saturating_add(1));
            ActiveEntries::<T>::mutate(|c| *c = c.saturating_add(1));
            TotalStorageBytes::<T>::mutate(|b| *b = b.saturating_add(size_bytes as u64));

            Self::deposit_event(Event::DataStored {
                epoch,
                actor,
                key,
                data_type,
                size: size_bytes,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::update_data())]
        pub fn update_data(
            origin: OriginFor<T>,
            epoch: EpochId,
            key: DataKey,
            new_data_hash: H256,
            new_size: u32,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(who);

            ensure!(
                new_size <= T::MaxDataSize::get(),
                Error::<T>::DataTooLarge
            );

            EphemeralData::<T>::try_mutate((epoch, actor, key), |entry| -> DispatchResult {
                let e = entry.as_mut().ok_or(Error::<T>::DataNotFound)?;

                ensure!(e.owner == actor, Error::<T>::NotDataOwner);
                ensure!(e.status == StorageStatus::Active, Error::<T>::DataExpired);

                let old_size = e.size_bytes;
                let size_diff = new_size as i64 - old_size as i64;

                if size_diff > 0 {
                    Self::check_actor_quota(actor, size_diff as u32)?;
                }

                e.data_hash = new_data_hash;
                e.size_bytes = new_size;

                if size_diff != 0 {
                    if size_diff > 0 {
                        Self::update_actor_quota(actor, size_diff as u32, true);
                        Self::update_epoch_storage(epoch, size_diff as u32, true);
                        TotalStorageBytes::<T>::mutate(|b| *b = b.saturating_add(size_diff as u64));
                    } else {
                        let abs_diff = (-size_diff) as u32;
                        Self::update_actor_quota(actor, abs_diff, false);
                        Self::update_epoch_storage(epoch, abs_diff, false);
                        TotalStorageBytes::<T>::mutate(|b| *b = b.saturating_sub(abs_diff as u64));
                    }
                }

                Self::deposit_event(Event::DataUpdated { epoch, actor, key });

                Ok(())
            })
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::delete_data())]
        pub fn delete_data(
            origin: OriginFor<T>,
            epoch: EpochId,
            key: DataKey,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(who);

            let entry = EphemeralData::<T>::get((epoch, actor, key))
                .ok_or(Error::<T>::DataNotFound)?;

            ensure!(entry.owner == actor, Error::<T>::NotDataOwner);

            let size = entry.size_bytes;

            EphemeralData::<T>::remove((epoch, actor, key));
            ActorEntries::<T>::remove(actor, key);
            EpochEntries::<T>::remove(epoch, key);

            Self::update_actor_quota(actor, size, false);
            Self::update_epoch_storage(epoch, size, false);

            ActiveEntries::<T>::mutate(|c| *c = c.saturating_sub(1));
            TotalStorageBytes::<T>::mutate(|b| *b = b.saturating_sub(size as u64));

            Self::deposit_event(Event::DataDeleted { epoch, actor, key });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::set_quota())]
        pub fn set_quota(
            origin: OriginFor<T>,
            actor: ActorId,
            max_entries: u32,
            max_bytes: u64,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();

            StorageQuotas::<T>::mutate(actor, |quota| {
                if let Some(ref mut q) = quota {
                    q.max_entries = max_entries;
                    q.max_bytes = max_bytes;
                    q.last_updated = block_number;
                } else {
                    *quota = Some(StorageQuota {
                        actor,
                        max_entries,
                        max_bytes,
                        used_entries: 0,
                        used_bytes: 0,
                        last_updated: block_number,
                    });
                }
            });

            Self::deposit_event(Event::QuotaUpdated {
                actor,
                max_entries,
                max_bytes,
            });

            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::finalize_epoch())]
        pub fn finalize_epoch(origin: OriginFor<T>, epoch: EpochId) -> DispatchResult {
            ensure_root(origin)?;

            EpochStorageInfo::<T>::try_mutate(epoch, |storage| -> DispatchResult {
                let s = storage.as_mut().ok_or(Error::<T>::EpochNotActive)?;

                s.finalized = true;

                Self::deposit_event(Event::EpochFinalized {
                    epoch,
                    entries: s.entry_count,
                });

                Ok(())
            })
        }
    }

    impl<T: Config> Pallet<T> {
        fn account_to_actor(account: T::AccountId) -> ActorId {
            let encoded = account.encode();
            let mut bytes = [0u8; 32];
            let len = encoded.len().min(32);
            bytes[..len].copy_from_slice(&encoded[..len]);
            ActorId::from_raw(bytes)
        }

        fn check_actor_quota(actor: ActorId, additional_bytes: u32) -> DispatchResult {
            if let Some(quota) = StorageQuotas::<T>::get(actor) {
                ensure!(
                    quota.used_entries < quota.max_entries,
                    Error::<T>::QuotaExceeded
                );
                ensure!(
                    quota.used_bytes.saturating_add(additional_bytes as u64) <= quota.max_bytes,
                    Error::<T>::QuotaExceeded
                );
            }
            Ok(())
        }

        fn check_epoch_quota(epoch: EpochId) -> DispatchResult {
            if let Some(storage) = EpochStorageInfo::<T>::get(epoch) {
                ensure!(
                    storage.entry_count < T::MaxEntriesPerEpoch::get(),
                    Error::<T>::EpochQuotaExceeded
                );
            }
            Ok(())
        }

        fn update_actor_quota(actor: ActorId, bytes: u32, increase: bool) {
            let block_number = frame_system::Pallet::<T>::block_number();

            StorageQuotas::<T>::mutate(actor, |quota| {
                if let Some(ref mut q) = quota {
                    if increase {
                        q.used_entries = q.used_entries.saturating_add(1);
                        q.used_bytes = q.used_bytes.saturating_add(bytes as u64);
                    } else {
                        q.used_entries = q.used_entries.saturating_sub(1);
                        q.used_bytes = q.used_bytes.saturating_sub(bytes as u64);
                    }
                    q.last_updated = block_number;
                } else if increase {
                    *quota = Some(StorageQuota {
                        actor,
                        max_entries: T::MaxEntriesPerActor::get(),
                        max_bytes: T::MaxDataSize::get() as u64 * T::MaxEntriesPerActor::get() as u64,
                        used_entries: 1,
                        used_bytes: bytes as u64,
                        last_updated: block_number,
                    });
                }
            });
        }

        fn update_epoch_storage(epoch: EpochId, bytes: u32, increase: bool) {
            let block_number = frame_system::Pallet::<T>::block_number();

            EpochStorageInfo::<T>::mutate(epoch, |storage| {
                if let Some(ref mut s) = storage {
                    if increase {
                        s.entry_count = s.entry_count.saturating_add(1);
                        s.total_bytes = s.total_bytes.saturating_add(bytes as u64);
                    } else {
                        s.entry_count = s.entry_count.saturating_sub(1);
                        s.total_bytes = s.total_bytes.saturating_sub(bytes as u64);
                    }
                } else if increase {
                    *storage = Some(EpochStorage {
                        epoch,
                        entry_count: 1,
                        total_bytes: bytes as u64,
                        created_at: block_number,
                        finalized: false,
                    });
                }
            });
        }

        fn cleanup_expired_entries(current_block: BlockNumberFor<T>) -> Weight {
            let mut cleaned = 0u32;
            let max_cleanup = 10u32;

            for ((epoch, actor, key), entry) in EphemeralData::<T>::iter() {
                if cleaned >= max_cleanup {
                    break;
                }

                if let Some(expires_at) = entry.expires_at {
                    if current_block >= expires_at && entry.status == StorageStatus::Active {
                        let size = entry.size_bytes;

                        EphemeralData::<T>::mutate((epoch, actor, key), |e| {
                            if let Some(ref mut entry) = e {
                                entry.status = StorageStatus::Expired;
                            }
                        });

                        ActiveEntries::<T>::mutate(|c| *c = c.saturating_sub(1));
                        TotalStorageBytes::<T>::mutate(|b| *b = b.saturating_sub(size as u64));

                        Self::deposit_event(Event::DataExpired { epoch, actor, key });

                        cleaned = cleaned.saturating_add(1);
                    }
                }
            }

            Weight::from_parts(cleaned as u64 * 10_000, 0)
        }

        pub fn get_entry(epoch: EpochId, actor: ActorId, key: DataKey) -> Option<EphemeralEntry<T>> {
            EphemeralData::<T>::get((epoch, actor, key))
        }

        pub fn get_actor_entry_count(actor: ActorId) -> u32 {
            StorageQuotas::<T>::get(actor)
                .map(|q| q.used_entries)
                .unwrap_or(0)
        }

        pub fn get_epoch_entry_count(epoch: EpochId) -> u32 {
            EpochStorageInfo::<T>::get(epoch)
                .map(|s| s.entry_count)
                .unwrap_or(0)
        }

        pub fn is_epoch_finalized(epoch: EpochId) -> bool {
            EpochStorageInfo::<T>::get(epoch)
                .map(|s| s.finalized)
                .unwrap_or(false)
        }

        pub fn total_entries() -> u64 {
            EntryCount::<T>::get()
        }

        pub fn total_active() -> u64 {
            ActiveEntries::<T>::get()
        }

        pub fn total_bytes() -> u64 {
            TotalStorageBytes::<T>::get()
        }
    }
}
