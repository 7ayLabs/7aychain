#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::expect_used)]

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
    };
    use frame_system::pallet_prelude::*;
    use seveny_primitives::types::{EpochId, EpochState};
    use sp_runtime::traits::Saturating;

    use crate::WeightInfo;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;

        #[pallet::constant]
        type EpochDuration: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MinEpochDuration: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MaxEpochDuration: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type GracePeriod: Get<BlockNumberFor<Self>>;
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct EpochMetadata<BlockNumber> {
        pub id: EpochId,
        pub state: EpochState,
        pub start_block: BlockNumber,
        pub end_block: BlockNumber,
        pub finalized_block: Option<BlockNumber>,
        pub participant_count: u32,
    }

    impl<BlockNumber: Default + Copy> Default for EpochMetadata<BlockNumber> {
        fn default() -> Self {
            Self {
                id: EpochId::new(0),
                state: EpochState::Scheduled,
                start_block: BlockNumber::default(),
                end_block: BlockNumber::default(),
                finalized_block: None,
                participant_count: 0,
            }
        }
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct EpochScheduleConfig<BlockNumber> {
        pub duration: BlockNumber,
        pub grace_period: BlockNumber,
        pub auto_transition: bool,
    }

    impl<BlockNumber: Default> Default for EpochScheduleConfig<BlockNumber> {
        fn default() -> Self {
            Self {
                duration: BlockNumber::default(),
                grace_period: BlockNumber::default(),
                auto_transition: true,
            }
        }
    }

    #[pallet::storage]
    #[pallet::getter(fn current_epoch)]
    pub type CurrentEpoch<T: Config> = StorageValue<_, EpochId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn epoch_info)]
    pub type EpochInfo<T: Config> =
        StorageMap<_, Blake2_128Concat, EpochId, EpochMetadata<BlockNumberFor<T>>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn epoch_schedule)]
    pub type EpochSchedule<T: Config> =
        StorageValue<_, EpochScheduleConfig<BlockNumberFor<T>>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn epoch_count)]
    pub type EpochCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn last_finalized_epoch)]
    pub type LastFinalizedEpoch<T: Config> = StorageValue<_, EpochId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn epoch_participants)]
    pub type EpochParticipants<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        EpochId,
        Blake2_128Concat,
        T::AccountId,
        bool,
        ValueQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        EpochScheduled {
            epoch_id: EpochId,
            start_block: BlockNumberFor<T>,
            end_block: BlockNumberFor<T>,
        },
        EpochStarted {
            epoch_id: EpochId,
            block_number: BlockNumberFor<T>,
        },
        EpochClosed {
            epoch_id: EpochId,
            block_number: BlockNumberFor<T>,
        },
        EpochFinalized {
            epoch_id: EpochId,
            block_number: BlockNumberFor<T>,
            participant_count: u32,
        },
        ParticipantRegistered {
            epoch_id: EpochId,
            participant: T::AccountId,
        },
        EpochScheduleUpdated {
            duration: BlockNumberFor<T>,
            grace_period: BlockNumberFor<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        EpochNotFound,
        EpochNotActive,
        EpochNotClosed,
        EpochAlreadyFinalized,
        EpochImmutable,
        InvalidEpochTransition,
        EpochSequenceGap,
        InvalidEpochDuration,
        EpochNotScheduled,
        ParticipantAlreadyRegistered,
        EpochExpired,
        GracePeriodNotElapsed,
        InvalidScheduleConfig,
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub initial_epoch_duration: BlockNumberFor<T>,
        pub initial_grace_period: BlockNumberFor<T>,
        pub auto_transition: bool,
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            let schedule = EpochScheduleConfig {
                duration: self.initial_epoch_duration,
                grace_period: self.initial_grace_period,
                auto_transition: self.auto_transition,
            };
            EpochSchedule::<T>::put(schedule);

            let genesis_epoch = EpochId::new(1);
            let start_block = BlockNumberFor::<T>::from(1u32);
            let end_block = start_block.saturating_add(self.initial_epoch_duration);

            let metadata = EpochMetadata {
                id: genesis_epoch,
                state: EpochState::Active,
                start_block,
                end_block,
                finalized_block: None,
                participant_count: 0,
            };

            EpochInfo::<T>::insert(genesis_epoch, metadata);
            CurrentEpoch::<T>::put(genesis_epoch);
            EpochCount::<T>::put(1u64);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            let schedule = EpochSchedule::<T>::get();
            if !schedule.auto_transition {
                return T::DbWeight::get().reads(1);
            }

            let current_epoch_id = CurrentEpoch::<T>::get();
            if let Some(mut metadata) = EpochInfo::<T>::get(current_epoch_id) {
                if metadata.state == EpochState::Active && n >= metadata.end_block {
                    metadata.state = EpochState::Closed;
                    EpochInfo::<T>::insert(current_epoch_id, metadata.clone());

                    Self::deposit_event(Event::EpochClosed {
                        epoch_id: current_epoch_id,
                        block_number: n,
                    });

                    if Self::schedule_next_epoch(n).is_some() {
                        return T::DbWeight::get().reads_writes(3, 3);
                    }
                }

                if metadata.state == EpochState::Closed {
                    let grace_end = metadata.end_block.saturating_add(schedule.grace_period);
                    if n >= grace_end
                        && Self::try_start_next_epoch(n).is_ok() {
                            return T::DbWeight::get().reads_writes(4, 2);
                        }
                }
            }

            T::DbWeight::get().reads(2)
        }

        fn on_finalize(_n: BlockNumberFor<T>) {}
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::schedule_epoch())]
        pub fn schedule_epoch(
            origin: OriginFor<T>,
            start_block: BlockNumberFor<T>,
            duration: BlockNumberFor<T>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(
                duration >= T::MinEpochDuration::get() && duration <= T::MaxEpochDuration::get(),
                Error::<T>::InvalidEpochDuration
            );

            let current_count = EpochCount::<T>::get();
            let next_epoch_id = EpochId::new(current_count.saturating_add(1));

            if current_count > 0 {
                let prev_epoch_id = EpochId::new(current_count);
                if let Some(prev_metadata) = EpochInfo::<T>::get(prev_epoch_id) {
                    ensure!(
                        start_block >= prev_metadata.end_block,
                        Error::<T>::EpochSequenceGap
                    );
                }
            }

            let end_block = start_block.saturating_add(duration);

            let metadata = EpochMetadata {
                id: next_epoch_id,
                state: EpochState::Scheduled,
                start_block,
                end_block,
                finalized_block: None,
                participant_count: 0,
            };

            EpochInfo::<T>::insert(next_epoch_id, metadata);
            EpochCount::<T>::put(next_epoch_id.inner());

            Self::deposit_event(Event::EpochScheduled {
                epoch_id: next_epoch_id,
                start_block,
                end_block,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::start_epoch())]
        pub fn start_epoch(origin: OriginFor<T>, epoch_id: EpochId) -> DispatchResult {
            ensure_root(origin)?;

            let mut metadata = EpochInfo::<T>::get(epoch_id).ok_or(Error::<T>::EpochNotFound)?;

            ensure!(
                metadata.state == EpochState::Scheduled,
                Error::<T>::EpochNotScheduled
            );

            Self::ensure_sequential_transition(&epoch_id)?;

            let block_number = frame_system::Pallet::<T>::block_number();
            metadata.state = EpochState::Active;
            metadata.start_block = block_number;

            EpochInfo::<T>::insert(epoch_id, metadata);
            CurrentEpoch::<T>::put(epoch_id);

            Self::deposit_event(Event::EpochStarted {
                epoch_id,
                block_number,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::close_epoch())]
        pub fn close_epoch(origin: OriginFor<T>, epoch_id: EpochId) -> DispatchResult {
            ensure_root(origin)?;

            let mut metadata = EpochInfo::<T>::get(epoch_id).ok_or(Error::<T>::EpochNotFound)?;

            ensure!(
                metadata.state == EpochState::Active,
                Error::<T>::EpochNotActive
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            metadata.state = EpochState::Closed;

            EpochInfo::<T>::insert(epoch_id, metadata);

            Self::deposit_event(Event::EpochClosed {
                epoch_id,
                block_number,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::finalize_epoch())]
        pub fn finalize_epoch(origin: OriginFor<T>, epoch_id: EpochId) -> DispatchResult {
            ensure_root(origin)?;

            let mut metadata = EpochInfo::<T>::get(epoch_id).ok_or(Error::<T>::EpochNotFound)?;

            ensure!(
                metadata.state == EpochState::Closed,
                Error::<T>::EpochNotClosed
            );

            Self::ensure_grace_period_elapsed(&metadata)?;

            let block_number = frame_system::Pallet::<T>::block_number();
            metadata.state = EpochState::Finalized;
            metadata.finalized_block = Some(block_number);

            EpochInfo::<T>::insert(epoch_id, metadata.clone());
            LastFinalizedEpoch::<T>::put(epoch_id);

            Self::deposit_event(Event::EpochFinalized {
                epoch_id,
                block_number,
                participant_count: metadata.participant_count,
            });

            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::register_participant())]
        pub fn register_participant(origin: OriginFor<T>, epoch_id: EpochId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut metadata = EpochInfo::<T>::get(epoch_id).ok_or(Error::<T>::EpochNotFound)?;

            ensure!(
                metadata.state == EpochState::Active,
                Error::<T>::EpochNotActive
            );

            ensure!(
                !EpochParticipants::<T>::get(epoch_id, &who),
                Error::<T>::ParticipantAlreadyRegistered
            );

            EpochParticipants::<T>::insert(epoch_id, &who, true);
            metadata.participant_count = metadata.participant_count.saturating_add(1);
            EpochInfo::<T>::insert(epoch_id, metadata);

            Self::deposit_event(Event::ParticipantRegistered {
                epoch_id,
                participant: who,
            });

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::update_schedule())]
        pub fn update_schedule(
            origin: OriginFor<T>,
            duration: BlockNumberFor<T>,
            grace_period: BlockNumberFor<T>,
            auto_transition: bool,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(
                duration >= T::MinEpochDuration::get() && duration <= T::MaxEpochDuration::get(),
                Error::<T>::InvalidScheduleConfig
            );

            let schedule = EpochScheduleConfig {
                duration,
                grace_period,
                auto_transition,
            };

            EpochSchedule::<T>::put(schedule);

            Self::deposit_event(Event::EpochScheduleUpdated {
                duration,
                grace_period,
            });

            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::force_transition())]
        pub fn force_transition(
            origin: OriginFor<T>,
            epoch_id: EpochId,
            new_state: EpochState,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut metadata = EpochInfo::<T>::get(epoch_id).ok_or(Error::<T>::EpochNotFound)?;

            ensure!(
                metadata.state != EpochState::Finalized,
                Error::<T>::EpochImmutable
            );

            ensure!(
                metadata.state.can_transition_to(&new_state),
                Error::<T>::InvalidEpochTransition
            );

            let block_number = frame_system::Pallet::<T>::block_number();

            metadata.state = new_state;
            if new_state == EpochState::Finalized {
                metadata.finalized_block = Some(block_number);
                LastFinalizedEpoch::<T>::put(epoch_id);
            }

            EpochInfo::<T>::insert(epoch_id, metadata);

            if new_state == EpochState::Active {
                CurrentEpoch::<T>::put(epoch_id);
            }

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn ensure_sequential_transition(epoch_id: &EpochId) -> DispatchResult {
            let current = CurrentEpoch::<T>::get();

            if epoch_id.inner() > 1 {
                let expected_next = current.next();
                ensure!(*epoch_id == expected_next, Error::<T>::EpochSequenceGap);

                if let Some(current_metadata) = EpochInfo::<T>::get(current) {
                    ensure!(
                        current_metadata.state == EpochState::Closed
                            || current_metadata.state == EpochState::Finalized,
                        Error::<T>::InvalidEpochTransition
                    );
                }
            }

            Ok(())
        }

        fn ensure_grace_period_elapsed(
            metadata: &EpochMetadata<BlockNumberFor<T>>,
        ) -> DispatchResult {
            let schedule = EpochSchedule::<T>::get();
            let block_number = frame_system::Pallet::<T>::block_number();
            let grace_end = metadata.end_block.saturating_add(schedule.grace_period);

            ensure!(block_number >= grace_end, Error::<T>::GracePeriodNotElapsed);

            Ok(())
        }

        fn schedule_next_epoch(current_block: BlockNumberFor<T>) -> Option<EpochId> {
            let schedule = EpochSchedule::<T>::get();
            let current_count = EpochCount::<T>::get();
            let next_epoch_id = EpochId::new(current_count.saturating_add(1));

            let start_block = current_block.saturating_add(schedule.grace_period);
            let end_block = start_block.saturating_add(schedule.duration);

            let metadata = EpochMetadata {
                id: next_epoch_id,
                state: EpochState::Scheduled,
                start_block,
                end_block,
                finalized_block: None,
                participant_count: 0,
            };

            EpochInfo::<T>::insert(next_epoch_id, metadata);
            EpochCount::<T>::put(next_epoch_id.inner());

            Self::deposit_event(Event::EpochScheduled {
                epoch_id: next_epoch_id,
                start_block,
                end_block,
            });

            Some(next_epoch_id)
        }

        fn try_start_next_epoch(current_block: BlockNumberFor<T>) -> DispatchResult {
            let current_epoch_id = CurrentEpoch::<T>::get();
            let next_epoch_id = current_epoch_id.next();

            if let Some(mut next_metadata) = EpochInfo::<T>::get(next_epoch_id) {
                if next_metadata.state == EpochState::Scheduled {
                    next_metadata.state = EpochState::Active;
                    next_metadata.start_block = current_block;

                    EpochInfo::<T>::insert(next_epoch_id, next_metadata);
                    CurrentEpoch::<T>::put(next_epoch_id);

                    Self::deposit_event(Event::EpochStarted {
                        epoch_id: next_epoch_id,
                        block_number: current_block,
                    });

                    return Ok(());
                }
            }

            Err(Error::<T>::EpochNotScheduled.into())
        }

        pub fn get_epoch_state(epoch_id: EpochId) -> Option<EpochState> {
            EpochInfo::<T>::get(epoch_id).map(|m| m.state)
        }

        pub fn is_epoch_active(epoch_id: EpochId) -> bool {
            EpochInfo::<T>::get(epoch_id)
                .map(|m| m.state == EpochState::Active)
                .unwrap_or(false)
        }

        pub fn is_participant(epoch_id: EpochId, account: &T::AccountId) -> bool {
            EpochParticipants::<T>::get(epoch_id, account)
        }

        pub fn get_current_epoch_metadata() -> Option<EpochMetadata<BlockNumberFor<T>>> {
            let current = CurrentEpoch::<T>::get();
            EpochInfo::<T>::get(current)
        }
    }
}
