#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Get, StorageVersion},
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use seveny_primitives::types::ActorId;
    use sp_runtime::Saturating;
    use alloc::vec::Vec;

    use crate::WeightInfo;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct PathId(pub u64);

    impl PathId {
        pub const fn new(id: u64) -> Self {
            Self(id)
        }

        pub const fn inner(self) -> u64 {
            self.0
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct HopId(pub u64);

    impl HopId {
        pub const fn new(id: u64) -> Self {
            Self(id)
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub enum PathStatus {
        Initiated,
        InProgress,
        AwaitingReturn,
        Completed,
        TimedOut,
        Failed,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub enum HopDirection {
        Outbound,
        Return,
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct BoomerangPath<T: Config> {
        pub id: PathId,
        pub initiator: ActorId,
        pub target: ActorId,
        pub status: PathStatus,
        pub outbound_hops: u32,
        pub return_hops: u32,
        pub created_at: BlockNumberFor<T>,
        pub timeout_at: BlockNumberFor<T>,
        pub extended_timeout_at: Option<BlockNumberFor<T>>,
        pub completed_at: Option<BlockNumberFor<T>>,
        pub verification_hash: Option<sp_core::H256>,
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct PathHop<T: Config> {
        pub id: HopId,
        pub path_id: PathId,
        pub from_actor: ActorId,
        pub to_actor: ActorId,
        pub direction: HopDirection,
        pub hop_index: u32,
        pub recorded_at: BlockNumberFor<T>,
        pub signature_hash: sp_core::H256,
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
        type WeightInfo: WeightInfo;

        #[pallet::constant]
        type BoomerangTimeoutBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MaxExtensionBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MaxHopsPerPath: Get<u32>;

        #[pallet::constant]
        type MaxActivePaths: Get<u32>;
    }

    #[pallet::storage]
    #[pallet::getter(fn paths)]
    pub type Paths<T: Config> =
        StorageMap<_, Blake2_128Concat, PathId, BoomerangPath<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn path_count)]
    pub type PathCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn hop_count)]
    pub type HopCount<T: Config> = StorageMap<_, Blake2_128Concat, PathId, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn path_hops)]
    pub type PathHops<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        PathId,
        Blake2_128Concat,
        HopId,
        PathHop<T>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn active_paths)]
    pub type ActivePaths<T: Config> =
        StorageValue<_, BoundedVec<PathId, T::MaxActivePaths>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn actor_paths)]
    pub type ActorPaths<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ActorId,
        BoundedVec<PathId, T::MaxActivePaths>,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        PathInitiated {
            path_id: PathId,
            initiator: ActorId,
            target: ActorId,
            timeout_at: BlockNumberFor<T>,
        },
        HopRecorded {
            path_id: PathId,
            hop_id: HopId,
            from_actor: ActorId,
            to_actor: ActorId,
            direction: HopDirection,
        },
        PathAwaitingReturn {
            path_id: PathId,
            outbound_hops: u32,
        },
        PathCompleted {
            path_id: PathId,
            total_hops: u32,
            verification_hash: sp_core::H256,
        },
        PathTimedOut {
            path_id: PathId,
        },
        PathExtended {
            path_id: PathId,
            new_timeout_at: BlockNumberFor<T>,
        },
        PathFailed {
            path_id: PathId,
            reason: PathFailureReason,
        },
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub enum PathFailureReason {
        InvalidHop,
        MismatchedReturn,
        VerificationFailed,
        MaxHopsExceeded,
    }

    #[pallet::error]
    pub enum Error<T> {
        PathNotFound,
        PathAlreadyCompleted,
        PathTimedOut,
        PathNotAwaitingReturn,
        InvalidHopSequence,
        MaxHopsReached,
        MaxActivePathsReached,
        NotAuthorized,
        ExtensionNotAllowed,
        AlreadyExtended,
        VerificationFailed,
        InvalidTarget,
        SelfPath,
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            PathCount::<T>::put(0u64);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut timed_out_count = 0u32;
            let active = ActivePaths::<T>::get();

            for path_id in active.iter() {
                if let Some(path) = Paths::<T>::get(path_id) {
                    let effective_timeout = path.extended_timeout_at.unwrap_or(path.timeout_at);
                    if now >= effective_timeout
                        && (path.status == PathStatus::InProgress
                            || path.status == PathStatus::AwaitingReturn
                            || path.status == PathStatus::Initiated)
                    {
                        Paths::<T>::mutate(path_id, |p| {
                            if let Some(ref mut pa) = p {
                                pa.status = PathStatus::TimedOut;
                            }
                        });
                        Self::deposit_event(Event::PathTimedOut { path_id: *path_id });
                        timed_out_count = timed_out_count.saturating_add(1);
                    }
                }
            }

            if timed_out_count > 0 {
                ActivePaths::<T>::mutate(|paths| {
                    paths.retain(|id| {
                        Paths::<T>::get(id)
                            .map(|p| {
                                p.status != PathStatus::TimedOut
                                    && p.status != PathStatus::Completed
                                    && p.status != PathStatus::Failed
                            })
                            .unwrap_or(false)
                    });
                });
            }

            T::DbWeight::get()
                .reads(active.len() as u64)
                .saturating_add(T::DbWeight::get().writes(timed_out_count.into()))
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::initiate_path())]
        pub fn initiate_path(origin: OriginFor<T>, target: ActorId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let initiator = Self::account_to_actor(&who);
            let block_number = frame_system::Pallet::<T>::block_number();

            ensure!(initiator != target, Error::<T>::SelfPath);

            let path_id = PathId::new(PathCount::<T>::get());
            PathCount::<T>::put(path_id.inner().saturating_add(1));

            let timeout_at = block_number.saturating_add(T::BoomerangTimeoutBlocks::get());

            let path = BoomerangPath {
                id: path_id,
                initiator,
                target,
                status: PathStatus::Initiated,
                outbound_hops: 0,
                return_hops: 0,
                created_at: block_number,
                timeout_at,
                extended_timeout_at: None,
                completed_at: None,
                verification_hash: None,
            };

            Paths::<T>::insert(path_id, path);

            ActivePaths::<T>::try_mutate(|paths| {
                paths
                    .try_push(path_id)
                    .map_err(|_| Error::<T>::MaxActivePathsReached)
            })?;

            ActorPaths::<T>::try_mutate(initiator, |paths| {
                paths
                    .try_push(path_id)
                    .map_err(|_| Error::<T>::MaxActivePathsReached)
            })?;

            Self::deposit_event(Event::PathInitiated {
                path_id,
                initiator,
                target,
                timeout_at,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::record_hop())]
        pub fn record_hop(
            origin: OriginFor<T>,
            path_id: PathId,
            to_actor: ActorId,
            signature_hash: sp_core::H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let from_actor = Self::account_to_actor(&who);
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut path = Paths::<T>::get(path_id).ok_or(Error::<T>::PathNotFound)?;

            let effective_timeout = path.extended_timeout_at.unwrap_or(path.timeout_at);
            ensure!(block_number < effective_timeout, Error::<T>::PathTimedOut);

            ensure!(
                path.status == PathStatus::Initiated
                    || path.status == PathStatus::InProgress
                    || path.status == PathStatus::AwaitingReturn,
                Error::<T>::PathAlreadyCompleted
            );

            let total_hops = path.outbound_hops.saturating_add(path.return_hops);
            ensure!(total_hops < T::MaxHopsPerPath::get(), Error::<T>::MaxHopsReached);

            let direction = if path.status == PathStatus::AwaitingReturn {
                HopDirection::Return
            } else {
                HopDirection::Outbound
            };

            let hop_count = HopCount::<T>::get(path_id);
            let hop_id = HopId::new(hop_count);
            HopCount::<T>::insert(path_id, hop_count.saturating_add(1));

            let hop_index = match direction {
                HopDirection::Outbound => path.outbound_hops,
                HopDirection::Return => path.return_hops,
            };

            let hop = PathHop {
                id: hop_id,
                path_id,
                from_actor,
                to_actor,
                direction,
                hop_index,
                recorded_at: block_number,
                signature_hash,
            };

            PathHops::<T>::insert(path_id, hop_id, hop);

            match direction {
                HopDirection::Outbound => {
                    path.outbound_hops = path.outbound_hops.saturating_add(1);
                    if path.status == PathStatus::Initiated {
                        path.status = PathStatus::InProgress;
                    }
                    if to_actor == path.target {
                        path.status = PathStatus::AwaitingReturn;
                        Self::deposit_event(Event::PathAwaitingReturn {
                            path_id,
                            outbound_hops: path.outbound_hops,
                        });
                    }
                }
                HopDirection::Return => {
                    path.return_hops = path.return_hops.saturating_add(1);
                    if to_actor == path.initiator {
                        path.status = PathStatus::Completed;
                        path.completed_at = Some(block_number);

                        let verification_hash = Self::compute_verification_hash(path_id);
                        path.verification_hash = Some(verification_hash);

                        ActivePaths::<T>::mutate(|paths| {
                            paths.retain(|id| *id != path_id);
                        });

                        Self::deposit_event(Event::PathCompleted {
                            path_id,
                            total_hops: path.outbound_hops.saturating_add(path.return_hops),
                            verification_hash,
                        });
                    }
                }
            }

            Paths::<T>::insert(path_id, path);

            Self::deposit_event(Event::HopRecorded {
                path_id,
                hop_id,
                from_actor,
                to_actor,
                direction,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::extend_timeout())]
        pub fn extend_timeout(origin: OriginFor<T>, path_id: PathId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut path = Paths::<T>::get(path_id).ok_or(Error::<T>::PathNotFound)?;

            ensure!(
                path.initiator == actor || path.target == actor,
                Error::<T>::NotAuthorized
            );
            ensure!(
                path.extended_timeout_at.is_none(),
                Error::<T>::AlreadyExtended
            );
            ensure!(
                path.status == PathStatus::InProgress || path.status == PathStatus::AwaitingReturn,
                Error::<T>::ExtensionNotAllowed
            );
            ensure!(block_number < path.timeout_at, Error::<T>::PathTimedOut);

            let new_timeout = path.timeout_at.saturating_add(T::MaxExtensionBlocks::get());
            path.extended_timeout_at = Some(new_timeout);

            Paths::<T>::insert(path_id, path);

            Self::deposit_event(Event::PathExtended {
                path_id,
                new_timeout_at: new_timeout,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::fail_path())]
        pub fn fail_path(
            origin: OriginFor<T>,
            path_id: PathId,
            reason: PathFailureReason,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut path = Paths::<T>::get(path_id).ok_or(Error::<T>::PathNotFound)?;

            ensure!(
                path.status != PathStatus::Completed && path.status != PathStatus::Failed,
                Error::<T>::PathAlreadyCompleted
            );

            path.status = PathStatus::Failed;
            Paths::<T>::insert(path_id, path);

            ActivePaths::<T>::mutate(|paths| {
                paths.retain(|id| *id != path_id);
            });

            Self::deposit_event(Event::PathFailed { path_id, reason });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn get_path(path_id: PathId) -> Option<BoomerangPath<T>> {
            Paths::<T>::get(path_id)
        }

        pub fn get_path_hops(path_id: PathId) -> Vec<PathHop<T>> {
            let hop_count = HopCount::<T>::get(path_id);
            (0..hop_count)
                .filter_map(|i| PathHops::<T>::get(path_id, HopId::new(i)))
                .collect()
        }

        pub fn is_path_active(path_id: PathId) -> bool {
            Paths::<T>::get(path_id)
                .map(|p| {
                    p.status == PathStatus::Initiated
                        || p.status == PathStatus::InProgress
                        || p.status == PathStatus::AwaitingReturn
                })
                .unwrap_or(false)
        }

        pub fn get_active_path_count() -> u32 {
            ActivePaths::<T>::get().len() as u32
        }

        pub fn is_path_complete(path_id: PathId) -> bool {
            Paths::<T>::get(path_id)
                .map(|p| p.status == PathStatus::Completed)
                .unwrap_or(false)
        }

        pub fn verify_path(path_id: PathId) -> bool {
            if let Some(path) = Paths::<T>::get(path_id) {
                if path.status != PathStatus::Completed {
                    return false;
                }
                if path.outbound_hops == 0 || path.return_hops == 0 {
                    return false;
                }
                path.verification_hash.is_some()
            } else {
                false
            }
        }

        fn compute_verification_hash(path_id: PathId) -> sp_core::H256 {
            use sp_runtime::traits::Hash;

            let hops = Self::get_path_hops(path_id);
            let mut data = Vec::new();

            for hop in hops {
                data.extend_from_slice(hop.from_actor.as_bytes());
                data.extend_from_slice(hop.to_actor.as_bytes());
                data.extend_from_slice(hop.signature_hash.as_bytes());
            }

            let hash = <T as frame_system::Config>::Hashing::hash(&data);
            let hash_bytes: [u8; 32] = hash.as_ref().try_into().unwrap_or([0u8; 32]);
            sp_core::H256(hash_bytes)
        }

        fn account_to_actor(account: &T::AccountId) -> ActorId {
            let encoded = account.encode();
            let mut bytes = [0u8; 32];
            let len = encoded.len().min(32);
            bytes[..len].copy_from_slice(&encoded[..len]);
            ActorId::from_raw(bytes)
        }
    }
}
