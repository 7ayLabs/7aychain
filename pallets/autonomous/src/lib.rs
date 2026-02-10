#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

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
use sp_runtime::traits::Hash;
use alloc::vec::Vec;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Default, Hash,
)]
pub struct PatternId(pub u64);

impl PatternId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Default, Hash,
)]
pub struct BehaviorId(pub u64);

impl BehaviorId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub enum BehaviorType {
    PresencePattern,
    InteractionPattern,
    TemporalPattern,
    TransactionPattern,
    NetworkPattern,
    Custom,
}

impl Default for BehaviorType {
    fn default() -> Self {
        Self::PresencePattern
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub enum PatternClassification {
    Normal,
    PotentiallyAutomated,
    Automated,
    Anomalous,
    Malicious,
}

impl Default for PatternClassification {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub enum AutonomousStatus {
    Unknown,
    Human,
    Suspected,
    Confirmed,
    UnderReview,
    Flagged,
}

impl Default for AutonomousStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Behavior<T: Config> {
    pub id: BehaviorId,
    pub actor: ActorId,
    pub behavior_type: BehaviorType,
    pub data_hash: H256,
    pub recorded_at: BlockNumberFor<T>,
    pub matched_pattern: Option<PatternId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Pattern<T: Config> {
    pub id: PatternId,
    pub behavior_type: BehaviorType,
    pub signature_hash: H256,
    pub classification: PatternClassification,
    pub occurrence_count: u32,
    pub confidence_score: u8,
    pub first_detected: BlockNumberFor<T>,
    pub last_observed: BlockNumberFor<T>,
    pub threshold_met: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct ActorProfile<T: Config> {
    pub actor: ActorId,
    pub status: AutonomousStatus,
    pub behavior_count: u32,
    pub pattern_count: u32,
    pub automation_score: u8,
    pub created_at: BlockNumberFor<T>,
    pub updated_at: BlockNumberFor<T>,
    pub flag_count: u32,
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
        type PatternThreshold: Get<u32>;

        #[pallet::constant]
        type MaxBehaviorsPerActor: Get<u32>;

        #[pallet::constant]
        type MaxPatterns: Get<u32>;

        #[pallet::constant]
        type BehaviorExpiryBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type ScoreIncreasePerMatch: Get<u8>;
    }

    #[pallet::storage]
    #[pallet::getter(fn behavior_count)]
    pub type BehaviorCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pattern_count)]
    pub type PatternCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn patterns)]
    pub type Patterns<T: Config> = StorageMap<_, Blake2_128Concat, PatternId, Pattern<T>>;

    #[pallet::storage]
    #[pallet::getter(fn pattern_by_hash)]
    pub type PatternByHash<T: Config> = StorageMap<_, Blake2_128Concat, H256, PatternId>;

    #[pallet::storage]
    #[pallet::getter(fn actor_behaviors)]
    pub type ActorBehaviors<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ActorId,
        Blake2_128Concat,
        BehaviorId,
        Behavior<T>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn actor_profiles)]
    pub type ActorProfiles<T: Config> = StorageMap<_, Blake2_128Concat, ActorId, ActorProfile<T>>;

    #[pallet::storage]
    #[pallet::getter(fn pattern_actors)]
    pub type PatternActors<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        PatternId,
        Blake2_128Concat,
        ActorId,
        u32,
    >;

    #[pallet::storage]
    #[pallet::getter(fn behavior_count_per_actor)]
    pub type BehaviorCountPerActor<T: Config> =
        StorageMap<_, Blake2_128Concat, ActorId, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn active_pattern_count)]
    pub type ActivePatternCount<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            BehaviorCount::<T>::put(0u64);
            PatternCount::<T>::put(0u64);
            ActivePatternCount::<T>::put(0u32);
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        BehaviorRecorded {
            behavior_id: BehaviorId,
            actor: ActorId,
            behavior_type: BehaviorType,
        },
        PatternDetected {
            pattern_id: PatternId,
            behavior_type: BehaviorType,
            signature_hash: H256,
        },
        PatternThresholdMet {
            pattern_id: PatternId,
            occurrence_count: u32,
        },
        PatternClassified {
            pattern_id: PatternId,
            classification: PatternClassification,
        },
        ProfileCreated {
            actor: ActorId,
        },
        StatusUpdated {
            actor: ActorId,
            old_status: AutonomousStatus,
            new_status: AutonomousStatus,
        },
        ActorFlagged {
            actor: ActorId,
            reason: H256,
        },
        BehaviorMatched {
            behavior_id: BehaviorId,
            pattern_id: PatternId,
            actor: ActorId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        PatternNotFound,
        BehaviorNotFound,
        ProfileNotFound,
        MaxBehaviorsReached,
        MaxPatternsReached,
        PatternAlreadyExists,
        InvalidBehaviorData,
        InvalidClassification,
        ProfileAlreadyExists,
        NotAuthorized,
        InvalidConfidenceScore,
        CannotFlagActor,
        BehaviorExpired,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::record_behavior())]
        pub fn record_behavior(
            origin: OriginFor<T>,
            actor: ActorId,
            behavior_type: BehaviorType,
            data_hash: H256,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let behavior_count = BehaviorCountPerActor::<T>::get(actor);
            ensure!(
                behavior_count < T::MaxBehaviorsPerActor::get(),
                Error::<T>::MaxBehaviorsReached
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            let behavior_id = Self::next_behavior_id();
            let behavior = Behavior {
                id: behavior_id,
                actor,
                behavior_type,
                data_hash,
                recorded_at: block_number,
                matched_pattern: None,
            };

            ActorBehaviors::<T>::insert(actor, behavior_id, behavior);
            BehaviorCountPerActor::<T>::mutate(actor, |count| *count = count.saturating_add(1));

            Self::ensure_profile_exists(actor, block_number);

            ActorProfiles::<T>::mutate(actor, |profile| {
                if let Some(ref mut p) = profile {
                    p.behavior_count = p.behavior_count.saturating_add(1);
                    p.updated_at = block_number;
                }
            });

            Self::try_match_pattern(behavior_id, actor, behavior_type, data_hash, block_number);

            Self::deposit_event(Event::BehaviorRecorded {
                behavior_id,
                actor,
                behavior_type,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::register_pattern())]
        pub fn register_pattern(
            origin: OriginFor<T>,
            behavior_type: BehaviorType,
            signature_hash: H256,
            classification: PatternClassification,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(
                !PatternByHash::<T>::contains_key(signature_hash),
                Error::<T>::PatternAlreadyExists
            );

            let active_count = ActivePatternCount::<T>::get();
            ensure!(
                active_count < T::MaxPatterns::get(),
                Error::<T>::MaxPatternsReached
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            let pattern_id = Self::next_pattern_id();

            let pattern = Pattern {
                id: pattern_id,
                behavior_type,
                signature_hash,
                classification,
                occurrence_count: 0,
                confidence_score: 0,
                first_detected: block_number,
                last_observed: block_number,
                threshold_met: false,
            };

            Patterns::<T>::insert(pattern_id, pattern);
            PatternByHash::<T>::insert(signature_hash, pattern_id);
            ActivePatternCount::<T>::mutate(|count| *count = count.saturating_add(1));

            Self::deposit_event(Event::PatternDetected {
                pattern_id,
                behavior_type,
                signature_hash,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::classify_pattern())]
        pub fn classify_pattern(
            origin: OriginFor<T>,
            pattern_id: PatternId,
            classification: PatternClassification,
            confidence_score: u8,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(confidence_score <= 100, Error::<T>::InvalidConfidenceScore);

            Patterns::<T>::try_mutate(pattern_id, |pattern| -> DispatchResult {
                let p = pattern.as_mut().ok_or(Error::<T>::PatternNotFound)?;
                p.classification = classification;
                p.confidence_score = confidence_score;
                Ok(())
            })?;

            Self::deposit_event(Event::PatternClassified {
                pattern_id,
                classification,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::update_status())]
        pub fn update_status(
            origin: OriginFor<T>,
            actor: ActorId,
            new_status: AutonomousStatus,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();

            ActorProfiles::<T>::try_mutate(actor, |profile| -> DispatchResult {
                let p = profile.as_mut().ok_or(Error::<T>::ProfileNotFound)?;
                let old_status = p.status;
                p.status = new_status;
                p.updated_at = block_number;

                Self::deposit_event(Event::StatusUpdated {
                    actor,
                    old_status,
                    new_status,
                });

                Ok(())
            })
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::flag_actor())]
        pub fn flag_actor(origin: OriginFor<T>, actor: ActorId, reason: H256) -> DispatchResult {
            ensure_root(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();

            ActorProfiles::<T>::try_mutate(actor, |profile| -> DispatchResult {
                let p = profile.as_mut().ok_or(Error::<T>::ProfileNotFound)?;

                ensure!(
                    p.status != AutonomousStatus::Flagged,
                    Error::<T>::CannotFlagActor
                );

                p.status = AutonomousStatus::Flagged;
                p.flag_count = p.flag_count.saturating_add(1);
                p.updated_at = block_number;

                Ok(())
            })?;

            Self::deposit_event(Event::ActorFlagged { actor, reason });

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::match_behavior())]
        pub fn match_behavior(
            origin: OriginFor<T>,
            behavior_id: BehaviorId,
            actor: ActorId,
            pattern_id: PatternId,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(
                Patterns::<T>::contains_key(pattern_id),
                Error::<T>::PatternNotFound
            );

            let block_number = frame_system::Pallet::<T>::block_number();

            ActorBehaviors::<T>::try_mutate(
                actor,
                behavior_id,
                |behavior| -> DispatchResult {
                    let b = behavior.as_mut().ok_or(Error::<T>::BehaviorNotFound)?;
                    b.matched_pattern = Some(pattern_id);
                    Ok(())
                },
            )?;

            Self::increment_pattern_occurrence(pattern_id, actor, block_number);

            Self::deposit_event(Event::BehaviorMatched {
                behavior_id,
                pattern_id,
                actor,
            });

            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::create_profile())]
        pub fn create_profile(origin: OriginFor<T>, actor: ActorId) -> DispatchResult {
            ensure_signed(origin)?;

            ensure!(
                !ActorProfiles::<T>::contains_key(actor),
                Error::<T>::ProfileAlreadyExists
            );

            let block_number = frame_system::Pallet::<T>::block_number();

            let profile = ActorProfile {
                actor,
                status: AutonomousStatus::Unknown,
                behavior_count: 0,
                pattern_count: 0,
                automation_score: 0,
                created_at: block_number,
                updated_at: block_number,
                flag_count: 0,
            };

            ActorProfiles::<T>::insert(actor, profile);

            Self::deposit_event(Event::ProfileCreated { actor });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn next_behavior_id() -> BehaviorId {
            let id = BehaviorCount::<T>::get();
            BehaviorCount::<T>::put(id.saturating_add(1));
            BehaviorId::new(id)
        }

        fn next_pattern_id() -> PatternId {
            let id = PatternCount::<T>::get();
            PatternCount::<T>::put(id.saturating_add(1));
            PatternId::new(id)
        }

        fn ensure_profile_exists(actor: ActorId, block_number: BlockNumberFor<T>) {
            if !ActorProfiles::<T>::contains_key(actor) {
                let profile = ActorProfile {
                    actor,
                    status: AutonomousStatus::Unknown,
                    behavior_count: 0,
                    pattern_count: 0,
                    automation_score: 0,
                    created_at: block_number,
                    updated_at: block_number,
                    flag_count: 0,
                };
                ActorProfiles::<T>::insert(actor, profile);
                Self::deposit_event(Event::ProfileCreated { actor });
            }
        }

        fn try_match_pattern(
            behavior_id: BehaviorId,
            actor: ActorId,
            behavior_type: BehaviorType,
            data_hash: H256,
            block_number: BlockNumberFor<T>,
        ) {
            let signature = Self::compute_pattern_signature(behavior_type, data_hash);

            if let Some(pattern_id) = PatternByHash::<T>::get(signature) {
                ActorBehaviors::<T>::mutate(actor, behavior_id, |behavior| {
                    if let Some(ref mut b) = behavior {
                        b.matched_pattern = Some(pattern_id);
                    }
                });

                Self::increment_pattern_occurrence(pattern_id, actor, block_number);

                Self::deposit_event(Event::BehaviorMatched {
                    behavior_id,
                    pattern_id,
                    actor,
                });
            }
        }

        fn increment_pattern_occurrence(
            pattern_id: PatternId,
            actor: ActorId,
            block_number: BlockNumberFor<T>,
        ) {
            Patterns::<T>::mutate(pattern_id, |pattern| {
                if let Some(ref mut p) = pattern {
                    p.occurrence_count = p.occurrence_count.saturating_add(1);
                    p.last_observed = block_number;

                    if !p.threshold_met && p.occurrence_count >= T::PatternThreshold::get() {
                        p.threshold_met = true;
                        Self::deposit_event(Event::PatternThresholdMet {
                            pattern_id,
                            occurrence_count: p.occurrence_count,
                        });
                    }
                }
            });

            let actor_count = PatternActors::<T>::get(pattern_id, actor).unwrap_or(0);
            PatternActors::<T>::insert(pattern_id, actor, actor_count.saturating_add(1));

            ActorProfiles::<T>::mutate(actor, |profile| {
                if let Some(ref mut p) = profile {
                    if actor_count == 0 {
                        p.pattern_count = p.pattern_count.saturating_add(1);
                    }
                    let score_increase = T::ScoreIncreasePerMatch::get();
                    p.automation_score = p.automation_score.saturating_add(score_increase).min(100);
                    p.updated_at = block_number;

                    Self::evaluate_status_change(p);
                }
            });
        }

        fn evaluate_status_change(profile: &mut ActorProfile<T>) {
            let new_status = match profile.automation_score {
                0..=20 => AutonomousStatus::Human,
                21..=50 => AutonomousStatus::Suspected,
                51..=100 => AutonomousStatus::Confirmed,
                _ => AutonomousStatus::Unknown,
            };

            if profile.status == AutonomousStatus::Unknown
                || (profile.status == AutonomousStatus::Human
                    && new_status != AutonomousStatus::Human)
                || (profile.status == AutonomousStatus::Suspected
                    && new_status == AutonomousStatus::Confirmed)
            {
                let old_status = profile.status;
                profile.status = new_status;
                Self::deposit_event(Event::StatusUpdated {
                    actor: profile.actor,
                    old_status,
                    new_status,
                });
            }
        }

        fn compute_pattern_signature(behavior_type: BehaviorType, data_hash: H256) -> H256 {
            let mut data = Vec::new();
            data.push(behavior_type as u8);
            data.extend_from_slice(data_hash.as_bytes());
            let hash = <T as frame_system::Config>::Hashing::hash(&data);
            let hash_bytes: [u8; 32] = hash.as_ref().try_into().unwrap_or([0u8; 32]);
            H256(hash_bytes)
        }

        pub fn is_autonomous(actor: ActorId) -> bool {
            ActorProfiles::<T>::get(actor)
                .is_some_and(|p| p.status == AutonomousStatus::Confirmed)
        }

        pub fn get_automation_score(actor: ActorId) -> u8 {
            ActorProfiles::<T>::get(actor)
                .map(|p| p.automation_score)
                .unwrap_or(0)
        }

        pub fn pattern_threshold_met(pattern_id: PatternId) -> bool {
            Patterns::<T>::get(pattern_id)
                .is_some_and(|p| p.threshold_met)
        }

        pub fn get_pattern_occurrences(pattern_id: PatternId) -> u32 {
            Patterns::<T>::get(pattern_id)
                .map(|p| p.occurrence_count)
                .unwrap_or(0)
        }

        pub fn get_actor_behaviors(actor: ActorId) -> Vec<Behavior<T>> {
            ActorBehaviors::<T>::iter_prefix(actor)
                .map(|(_, behavior)| behavior)
                .collect()
        }

        pub fn get_active_patterns() -> u32 {
            ActivePatternCount::<T>::get()
        }
    }
}
