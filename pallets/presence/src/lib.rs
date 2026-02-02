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
    use seveny_primitives::{
        types::{
            ActorId, BlockRef, EpochId, PresenceRecord, PresenceState, QuorumConfig, ValidatorId,
            Vote,
        },
        CryptoCommitment as Commitment,
    };
    use sp_runtime::traits::Hash;
    use sp_std::vec::Vec;

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
        type MaxVotesPerPresence: Get<u32>;

        #[pallet::constant]
        type DefaultQuorumThreshold: Get<u32>;

        #[pallet::constant]
        type DefaultQuorumTotal: Get<u32>;
    }

    #[pallet::storage]
    #[pallet::getter(fn presences)]
    pub type Presences<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        EpochId,
        Blake2_128Concat,
        ActorId,
        PresenceRecord<BlockNumberFor<T>>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn votes)]
    pub type Votes<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Blake2_128Concat, EpochId>,
            NMapKey<Blake2_128Concat, ActorId>,
            NMapKey<Blake2_128Concat, ValidatorId>,
        ),
        Vote,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn vote_count)]
    pub type VoteCount<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, EpochId, Blake2_128Concat, ActorId, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn presence_count)]
    pub type PresenceCount<T: Config> = StorageMap<_, Blake2_128Concat, EpochId, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn declarations)]
    pub type Declarations<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        EpochId,
        Blake2_128Concat,
        ActorId,
        Declaration<BlockNumberFor<T>>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn quorum_config)]
    pub type QuorumConfigStorage<T: Config> = StorageValue<_, QuorumConfig, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn active_validators)]
    pub type ActiveValidators<T: Config> =
        StorageMap<_, Blake2_128Concat, ValidatorId, bool, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn current_epoch)]
    pub type CurrentEpoch<T: Config> = StorageValue<_, EpochId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn epoch_active)]
    pub type EpochActive<T: Config> = StorageMap<_, Blake2_128Concat, EpochId, bool, ValueQuery>;

    #[derive(Clone, PartialEq, Eq, Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct Declaration<BlockNumber> {
        pub commitment: Commitment,
        pub declared_at: BlockNumber,
        pub block_ref: BlockRef,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        PresenceDeclared {
            actor: ActorId,
            epoch: EpochId,
            block_number: BlockNumberFor<T>,
        },
        PresenceVoted {
            validator: ValidatorId,
            actor: ActorId,
            epoch: EpochId,
            approve: bool,
        },
        PresenceValidated {
            actor: ActorId,
            epoch: EpochId,
            vote_count: u32,
        },
        PresenceFinalized {
            actor: ActorId,
            epoch: EpochId,
            block_number: BlockNumberFor<T>,
        },
        PresenceSlashed {
            actor: ActorId,
            epoch: EpochId,
        },
        QuorumConfigUpdated {
            threshold: u32,
            total: u32,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        DuplicatePresence,
        PresenceImmutable,
        InvalidStateTransition,
        UnauthorizedDeclaration,
        EpochExpired,
        QuorumNotMet,
        DuplicateVote,
        SlashedTerminal,
        PresenceNotFound,
        EpochNotActive,
        ValidatorNotActive,
        InvalidQuorumConfig,
        ActorNotFound,
        PresenceNotValidated,
        ArithmeticOverflow,
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        pub quorum_threshold: u32,
        pub quorum_total: u32,
        pub initial_validators: Vec<[u8; 32]>,
        pub initial_epoch: u64,
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            let config = QuorumConfig::new(self.quorum_threshold, self.quorum_total);
            QuorumConfigStorage::<T>::put(config);

            for validator_bytes in &self.initial_validators {
                let validator = ValidatorId::from_raw(*validator_bytes);
                ActiveValidators::<T>::insert(validator, true);
            }

            let epoch = EpochId::new(self.initial_epoch);
            CurrentEpoch::<T>::put(epoch);
            EpochActive::<T>::insert(epoch, true);
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::declare_presence())]
        pub fn declare_presence(origin: OriginFor<T>, epoch: EpochId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);
            let block_number = frame_system::Pallet::<T>::block_number();

            Self::ensure_epoch_active(&epoch)?;
            Self::ensure_no_duplicate_presence(&epoch, &actor)?;

            let record = PresenceRecord {
                actor,
                epoch,
                state: PresenceState::Declared,
                declared_at: Some(block_number),
                validated_at: None,
                finalized_at: None,
                vote_count: 0,
            };

            Presences::<T>::insert(epoch, actor, record);
            PresenceCount::<T>::mutate(epoch, |count| {
                *count = count.saturating_add(1);
            });

            Self::deposit_event(Event::PresenceDeclared {
                actor,
                epoch,
                block_number,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::declare_presence_with_commitment())]
        pub fn declare_presence_with_commitment(
            origin: OriginFor<T>,
            epoch: EpochId,
            commitment: Commitment,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);
            let block_number = frame_system::Pallet::<T>::block_number();
            let block_hash = frame_system::Pallet::<T>::block_hash(block_number);

            Self::ensure_epoch_active(&epoch)?;
            Self::ensure_no_duplicate_presence(&epoch, &actor)?;

            let block_ref = BlockRef::new(
                block_number.try_into().unwrap_or(0),
                sp_core::H256(block_hash.as_ref().try_into().unwrap_or([0u8; 32])),
            );

            let declaration = Declaration {
                commitment,
                declared_at: block_number,
                block_ref,
            };

            Declarations::<T>::insert(epoch, actor, declaration);

            let record = PresenceRecord {
                actor,
                epoch,
                state: PresenceState::Declared,
                declared_at: Some(block_number),
                validated_at: None,
                finalized_at: None,
                vote_count: 0,
            };

            Presences::<T>::insert(epoch, actor, record);
            PresenceCount::<T>::mutate(epoch, |count| {
                *count = count.saturating_add(1);
            });

            Self::deposit_event(Event::PresenceDeclared {
                actor,
                epoch,
                block_number,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::vote_presence())]
        pub fn vote_presence(
            origin: OriginFor<T>,
            actor: ActorId,
            epoch: EpochId,
            approve: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let validator = Self::account_to_validator(&who);
            let block_number = frame_system::Pallet::<T>::block_number();
            let block_hash = frame_system::Pallet::<T>::block_hash(block_number);

            Self::ensure_validator_active(&validator)?;
            Self::ensure_epoch_active(&epoch)?;
            Self::ensure_no_duplicate_vote(&epoch, &actor, &validator)?;

            let mut record =
                Presences::<T>::get(epoch, actor).ok_or(Error::<T>::PresenceNotFound)?;

            Self::ensure_not_terminal(&record.state)?;
            Self::ensure_valid_vote_state(&record.state)?;

            let block_ref = BlockRef::new(
                block_number.try_into().unwrap_or(0),
                sp_core::H256(block_hash.as_ref().try_into().unwrap_or([0u8; 32])),
            );

            let vote = Vote {
                validator,
                actor,
                epoch,
                block_ref,
                approve,
            };

            Votes::<T>::insert((epoch, actor, validator), vote);

            if approve {
                record.vote_count = record.vote_count.saturating_add(1);
                VoteCount::<T>::insert(epoch, actor, record.vote_count);

                let quorum = QuorumConfigStorage::<T>::get();
                if quorum.is_met(record.vote_count) && record.state == PresenceState::Declared {
                    record.state = PresenceState::Validated;
                    record.validated_at = Some(block_number);

                    Self::deposit_event(Event::PresenceValidated {
                        actor,
                        epoch,
                        vote_count: record.vote_count,
                    });
                }
            }

            Presences::<T>::insert(epoch, actor, record);

            Self::deposit_event(Event::PresenceVoted {
                validator,
                actor,
                epoch,
                approve,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::finalize_presence())]
        pub fn finalize_presence(
            origin: OriginFor<T>,
            actor: ActorId,
            epoch: EpochId,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut record =
                Presences::<T>::get(epoch, actor).ok_or(Error::<T>::PresenceNotFound)?;

            Self::ensure_not_terminal(&record.state)?;
            ensure!(
                record.state == PresenceState::Validated,
                Error::<T>::PresenceNotValidated
            );

            let quorum = QuorumConfigStorage::<T>::get();
            ensure!(quorum.is_met(record.vote_count), Error::<T>::QuorumNotMet);

            record.state = PresenceState::Finalized;
            record.finalized_at = Some(block_number);

            Presences::<T>::insert(epoch, actor, record);

            Self::deposit_event(Event::PresenceFinalized {
                actor,
                epoch,
                block_number,
            });

            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::slash_presence())]
        pub fn slash_presence(
            origin: OriginFor<T>,
            actor: ActorId,
            epoch: EpochId,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut record =
                Presences::<T>::get(epoch, actor).ok_or(Error::<T>::PresenceNotFound)?;

            Self::ensure_not_terminal(&record.state)?;

            record.state = PresenceState::Slashed;
            Presences::<T>::insert(epoch, actor, record);

            Self::deposit_event(Event::PresenceSlashed { actor, epoch });

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::set_quorum_config())]
        pub fn set_quorum_config(
            origin: OriginFor<T>,
            threshold: u32,
            total: u32,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let config = QuorumConfig::new(threshold, total);
            ensure!(config.is_valid(), Error::<T>::InvalidQuorumConfig);

            QuorumConfigStorage::<T>::put(config);

            Self::deposit_event(Event::QuorumConfigUpdated { threshold, total });

            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::set_validator_status())]
        pub fn set_validator_status(
            origin: OriginFor<T>,
            validator: ValidatorId,
            active: bool,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ActiveValidators::<T>::insert(validator, active);

            Ok(())
        }

        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::set_epoch_active())]
        pub fn set_epoch_active(
            origin: OriginFor<T>,
            epoch: EpochId,
            active: bool,
        ) -> DispatchResult {
            ensure_root(origin)?;

            EpochActive::<T>::insert(epoch, active);
            if active {
                CurrentEpoch::<T>::put(epoch);
            }

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn account_to_actor(account: &T::AccountId) -> ActorId {
            let hash = T::Hashing::hash_of(account);
            ActorId::from(sp_core::H256(hash.as_ref().try_into().unwrap_or([0u8; 32])))
        }

        fn account_to_validator(account: &T::AccountId) -> ValidatorId {
            let hash = T::Hashing::hash_of(account);
            ValidatorId::from(sp_core::H256(hash.as_ref().try_into().unwrap_or([0u8; 32])))
        }

        fn ensure_epoch_active(epoch: &EpochId) -> DispatchResult {
            ensure!(EpochActive::<T>::get(epoch), Error::<T>::EpochNotActive);
            Ok(())
        }

        fn ensure_no_duplicate_presence(epoch: &EpochId, actor: &ActorId) -> DispatchResult {
            ensure!(
                !Presences::<T>::contains_key(epoch, actor),
                Error::<T>::DuplicatePresence
            );
            Ok(())
        }

        fn ensure_validator_active(validator: &ValidatorId) -> DispatchResult {
            ensure!(
                ActiveValidators::<T>::get(validator),
                Error::<T>::ValidatorNotActive
            );
            Ok(())
        }

        fn ensure_no_duplicate_vote(
            epoch: &EpochId,
            actor: &ActorId,
            validator: &ValidatorId,
        ) -> DispatchResult {
            ensure!(
                !Votes::<T>::contains_key((epoch, actor, validator)),
                Error::<T>::DuplicateVote
            );
            Ok(())
        }

        fn ensure_not_terminal(state: &PresenceState) -> DispatchResult {
            ensure!(!state.is_terminal(), Error::<T>::PresenceImmutable);
            Ok(())
        }

        fn ensure_valid_vote_state(state: &PresenceState) -> DispatchResult {
            ensure!(
                matches!(state, PresenceState::Declared | PresenceState::Validated),
                Error::<T>::InvalidStateTransition
            );
            Ok(())
        }

        pub fn get_presence(
            epoch: EpochId,
            actor: ActorId,
        ) -> Option<PresenceRecord<BlockNumberFor<T>>> {
            Presences::<T>::get(epoch, actor)
        }

        pub fn get_vote(epoch: EpochId, actor: ActorId, validator: ValidatorId) -> Option<Vote> {
            Votes::<T>::get((epoch, actor, validator))
        }

        pub fn get_declaration(
            epoch: EpochId,
            actor: ActorId,
        ) -> Option<Declaration<BlockNumberFor<T>>> {
            Declarations::<T>::get(epoch, actor)
        }
    }
}
