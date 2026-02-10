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
    pub struct RelationshipId(pub u64);

    impl RelationshipId {
        pub const fn new(id: u64) -> Self {
            Self(id)
        }

        pub const fn inner(self) -> u64 {
            self.0
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct DiscoveryRequestId(pub u64);

    impl DiscoveryRequestId {
        pub const fn new(id: u64) -> Self {
            Self(id)
        }

        pub const fn inner(self) -> u64 {
            self.0
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub enum RelationshipType {
        Trust,
        Follow,
        Block,
        Collaborate,
        Verify,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub enum RelationshipStatus {
        Active,
        Pending,
        Revoked,
        Expired,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub enum DiscoveryStatus {
        Pending,
        Processing,
        Completed,
        Failed,
        RateLimited,
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct Relationship<T: Config> {
        pub id: RelationshipId,
        pub from_actor: ActorId,
        pub to_actor: ActorId,
        pub relationship_type: RelationshipType,
        pub status: RelationshipStatus,
        pub created_at: BlockNumberFor<T>,
        pub updated_at: BlockNumberFor<T>,
        pub expires_at: Option<BlockNumberFor<T>>,
        pub bidirectional: bool,
        pub trust_level: u8,
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct DiscoveryRequest<T: Config> {
        pub id: DiscoveryRequestId,
        pub requester: ActorId,
        pub target_criteria: DiscoveryCriteria,
        pub status: DiscoveryStatus,
        pub created_at: BlockNumberFor<T>,
        pub completed_at: Option<BlockNumberFor<T>>,
        pub results_count: u32,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct DiscoveryCriteria {
        pub min_trust_level: u8,
        pub relationship_type: Option<RelationshipType>,
        pub max_hops: u8,
        pub include_pending: bool,
    }

    impl Default for DiscoveryCriteria {
        fn default() -> Self {
            Self {
                min_trust_level: 0,
                relationship_type: None,
                max_hops: 2,
                include_pending: false,
            }
        }
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct SemanticProfile {
        pub actor: ActorId,
        pub total_relationships: u32,
        pub trust_score: u32,
        pub discovery_enabled: bool,
        pub last_activity_block: u64,
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
        type WeightInfo: WeightInfo;

        #[pallet::constant]
        type MaxRelationshipsPerActor: Get<u32>;

        #[pallet::constant]
        type MaxDiscoveryResults: Get<u32>;

        #[pallet::constant]
        type DiscoveryRateLimitBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type RelationshipExpiryBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MaxTrustLevel: Get<u8>;
    }

    #[pallet::storage]
    #[pallet::getter(fn relationships)]
    pub type Relationships<T: Config> =
        StorageMap<_, Blake2_128Concat, RelationshipId, Relationship<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn relationship_count)]
    pub type RelationshipCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn actor_relationships)]
    pub type ActorRelationships<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ActorId,
        BoundedVec<RelationshipId, T::MaxRelationshipsPerActor>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn relationship_index)]
    pub type RelationshipIndex<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ActorId,
        Blake2_128Concat,
        ActorId,
        RelationshipId,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn discovery_requests)]
    pub type DiscoveryRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, DiscoveryRequestId, DiscoveryRequest<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn discovery_count)]
    pub type DiscoveryCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn last_discovery_block)]
    pub type LastDiscoveryBlock<T: Config> =
        StorageMap<_, Blake2_128Concat, ActorId, BlockNumberFor<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn semantic_profiles)]
    pub type SemanticProfiles<T: Config> =
        StorageMap<_, Blake2_128Concat, ActorId, SemanticProfile, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pending_discovery)]
    pub type PendingDiscovery<T: Config> =
        StorageValue<_, BoundedVec<DiscoveryRequestId, T::MaxDiscoveryResults>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        RelationshipCreated {
            relationship_id: RelationshipId,
            from_actor: ActorId,
            to_actor: ActorId,
            relationship_type: RelationshipType,
        },
        RelationshipUpdated {
            relationship_id: RelationshipId,
            new_status: RelationshipStatus,
        },
        RelationshipRevoked {
            relationship_id: RelationshipId,
            revoked_by: ActorId,
        },
        TrustLevelChanged {
            relationship_id: RelationshipId,
            old_level: u8,
            new_level: u8,
        },
        DiscoveryRequested {
            request_id: DiscoveryRequestId,
            requester: ActorId,
        },
        DiscoveryCompleted {
            request_id: DiscoveryRequestId,
            results_count: u32,
        },
        DiscoveryRateLimited {
            requester: ActorId,
            next_allowed_block: BlockNumberFor<T>,
        },
        ProfileUpdated {
            actor: ActorId,
            discovery_enabled: bool,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        RelationshipNotFound,
        RelationshipAlreadyExists,
        RelationshipRevoked,
        NotAuthorized,
        MaxRelationshipsReached,
        InvalidTrustLevel,
        DiscoveryRateLimited,
        DiscoveryNotFound,
        SelfRelationship,
        InvalidRelationshipType,
        ProfileNotFound,
        RelationshipExpired,
        PendingDiscoveryFull,
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
            RelationshipCount::<T>::put(0u64);
            DiscoveryCount::<T>::put(0u64);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut expired_count = 0u32;

            for (id, relationship) in Relationships::<T>::iter() {
                if relationship.status == RelationshipStatus::Active {
                    if let Some(expires_at) = relationship.expires_at {
                        if now >= expires_at {
                            Relationships::<T>::mutate(id, |rel| {
                                if let Some(ref mut r) = rel {
                                    r.status = RelationshipStatus::Expired;
                                }
                            });
                            expired_count = expired_count.saturating_add(1);
                        }
                    }
                }
            }

            T::DbWeight::get()
                .reads(expired_count.into())
                .saturating_add(T::DbWeight::get().writes(expired_count.into()))
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_relationship())]
        pub fn create_relationship(
            origin: OriginFor<T>,
            to_actor: ActorId,
            relationship_type: RelationshipType,
            trust_level: u8,
            expires_at: Option<BlockNumberFor<T>>,
            bidirectional: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let from_actor = Self::account_to_actor(&who);
            let block_number = frame_system::Pallet::<T>::block_number();

            ensure!(from_actor != to_actor, Error::<T>::SelfRelationship);
            ensure!(
                trust_level <= T::MaxTrustLevel::get(),
                Error::<T>::InvalidTrustLevel
            );
            ensure!(
                RelationshipIndex::<T>::get(from_actor, to_actor).is_none(),
                Error::<T>::RelationshipAlreadyExists
            );

            let relationship_id = RelationshipId::new(RelationshipCount::<T>::get());
            RelationshipCount::<T>::put(relationship_id.inner().saturating_add(1));

            let relationship = Relationship {
                id: relationship_id,
                from_actor,
                to_actor,
                relationship_type,
                status: if bidirectional {
                    RelationshipStatus::Pending
                } else {
                    RelationshipStatus::Active
                },
                created_at: block_number,
                updated_at: block_number,
                expires_at,
                bidirectional,
                trust_level,
            };

            Relationships::<T>::insert(relationship_id, relationship);
            RelationshipIndex::<T>::insert(from_actor, to_actor, relationship_id);

            ActorRelationships::<T>::try_mutate(from_actor, |rels| {
                rels.try_push(relationship_id)
                    .map_err(|_| Error::<T>::MaxRelationshipsReached)
            })?;

            Self::update_profile_relationship_count(from_actor, block_number, true);

            Self::deposit_event(Event::RelationshipCreated {
                relationship_id,
                from_actor,
                to_actor,
                relationship_type,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::accept_relationship())]
        pub fn accept_relationship(
            origin: OriginFor<T>,
            relationship_id: RelationshipId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut relationship =
                Relationships::<T>::get(relationship_id).ok_or(Error::<T>::RelationshipNotFound)?;

            ensure!(
                relationship.to_actor == actor,
                Error::<T>::NotAuthorized
            );
            ensure!(
                relationship.status == RelationshipStatus::Pending,
                Error::<T>::RelationshipRevoked
            );

            relationship.status = RelationshipStatus::Active;
            relationship.updated_at = block_number;

            Relationships::<T>::insert(relationship_id, relationship.clone());

            ActorRelationships::<T>::try_mutate(actor, |rels| {
                rels.try_push(relationship_id)
                    .map_err(|_| Error::<T>::MaxRelationshipsReached)
            })?;

            Self::update_profile_relationship_count(actor, block_number, true);

            Self::deposit_event(Event::RelationshipUpdated {
                relationship_id,
                new_status: RelationshipStatus::Active,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::revoke_relationship())]
        pub fn revoke_relationship(
            origin: OriginFor<T>,
            relationship_id: RelationshipId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut relationship =
                Relationships::<T>::get(relationship_id).ok_or(Error::<T>::RelationshipNotFound)?;

            ensure!(
                relationship.from_actor == actor || relationship.to_actor == actor,
                Error::<T>::NotAuthorized
            );
            ensure!(
                relationship.status != RelationshipStatus::Revoked,
                Error::<T>::RelationshipRevoked
            );

            relationship.status = RelationshipStatus::Revoked;
            relationship.updated_at = block_number;

            Relationships::<T>::insert(relationship_id, relationship.clone());

            Self::update_profile_relationship_count(relationship.from_actor, block_number, false);
            if relationship.bidirectional && relationship.status == RelationshipStatus::Active {
                Self::update_profile_relationship_count(relationship.to_actor, block_number, false);
            }

            Self::deposit_event(Event::RelationshipRevoked {
                relationship_id,
                revoked_by: actor,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::update_trust_level())]
        pub fn update_trust_level(
            origin: OriginFor<T>,
            relationship_id: RelationshipId,
            new_trust_level: u8,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut relationship =
                Relationships::<T>::get(relationship_id).ok_or(Error::<T>::RelationshipNotFound)?;

            ensure!(
                relationship.from_actor == actor,
                Error::<T>::NotAuthorized
            );
            ensure!(
                relationship.status == RelationshipStatus::Active,
                Error::<T>::RelationshipRevoked
            );
            ensure!(
                new_trust_level <= T::MaxTrustLevel::get(),
                Error::<T>::InvalidTrustLevel
            );

            let old_level = relationship.trust_level;
            relationship.trust_level = new_trust_level;
            relationship.updated_at = block_number;

            Relationships::<T>::insert(relationship_id, relationship);

            Self::deposit_event(Event::TrustLevelChanged {
                relationship_id,
                old_level,
                new_level: new_trust_level,
            });

            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::request_discovery())]
        pub fn request_discovery(
            origin: OriginFor<T>,
            criteria: DiscoveryCriteria,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let requester = Self::account_to_actor(&who);
            let block_number = frame_system::Pallet::<T>::block_number();

            let last_discovery = LastDiscoveryBlock::<T>::get(requester);
            let rate_limit_blocks = T::DiscoveryRateLimitBlocks::get();

            if last_discovery > BlockNumberFor::<T>::default() {
                let next_allowed = last_discovery.saturating_add(rate_limit_blocks);
                if block_number < next_allowed {
                    Self::deposit_event(Event::DiscoveryRateLimited {
                        requester,
                        next_allowed_block: next_allowed,
                    });
                    return Err(Error::<T>::DiscoveryRateLimited.into());
                }
            }

            let request_id = DiscoveryRequestId::new(DiscoveryCount::<T>::get());
            DiscoveryCount::<T>::put(request_id.inner().saturating_add(1));

            let request = DiscoveryRequest {
                id: request_id,
                requester,
                target_criteria: criteria,
                status: DiscoveryStatus::Pending,
                created_at: block_number,
                completed_at: None,
                results_count: 0,
            };

            DiscoveryRequests::<T>::insert(request_id, request);
            LastDiscoveryBlock::<T>::insert(requester, block_number);

            PendingDiscovery::<T>::try_mutate(|pending| {
                pending
                    .try_push(request_id)
                    .map_err(|_| Error::<T>::PendingDiscoveryFull)
            })?;

            Self::deposit_event(Event::DiscoveryRequested {
                request_id,
                requester,
            });

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::update_profile())]
        pub fn update_profile(
            origin: OriginFor<T>,
            discovery_enabled: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);
            let block_number = frame_system::Pallet::<T>::block_number();

            let block_number_u64: u64 = block_number
                .try_into()
                .unwrap_or(0u64);

            SemanticProfiles::<T>::mutate(actor, |profile| {
                if let Some(ref mut p) = profile {
                    p.discovery_enabled = discovery_enabled;
                    p.last_activity_block = block_number_u64;
                } else {
                    *profile = Some(SemanticProfile {
                        actor,
                        total_relationships: 0,
                        trust_score: 0,
                        discovery_enabled,
                        last_activity_block: block_number_u64,
                    });
                }
            });

            Self::deposit_event(Event::ProfileUpdated {
                actor,
                discovery_enabled,
            });

            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::complete_discovery())]
        pub fn complete_discovery(
            origin: OriginFor<T>,
            request_id: DiscoveryRequestId,
            results_count: u32,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut request =
                DiscoveryRequests::<T>::get(request_id).ok_or(Error::<T>::DiscoveryNotFound)?;

            request.status = DiscoveryStatus::Completed;
            request.completed_at = Some(block_number);
            request.results_count = results_count;

            DiscoveryRequests::<T>::insert(request_id, request);

            PendingDiscovery::<T>::mutate(|pending| {
                pending.retain(|id| *id != request_id);
            });

            Self::deposit_event(Event::DiscoveryCompleted {
                request_id,
                results_count,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn get_relationship(relationship_id: RelationshipId) -> Option<Relationship<T>> {
            Relationships::<T>::get(relationship_id)
        }

        pub fn get_actor_relationships(actor: ActorId) -> Vec<Relationship<T>> {
            ActorRelationships::<T>::get(actor)
                .iter()
                .filter_map(|rel_id| Relationships::<T>::get(rel_id))
                .collect()
        }

        pub fn has_relationship(from: ActorId, to: ActorId) -> bool {
            RelationshipIndex::<T>::get(from, to).is_some()
        }

        pub fn get_trust_level(from: ActorId, to: ActorId) -> Option<u8> {
            RelationshipIndex::<T>::get(from, to)
                .and_then(|rel_id| Relationships::<T>::get(rel_id))
                .filter(|rel| rel.status == RelationshipStatus::Active)
                .map(|rel| rel.trust_level)
        }

        pub fn get_mutual_relationships(actor1: ActorId, actor2: ActorId) -> Option<(Relationship<T>, Relationship<T>)> {
            let rel1_id = RelationshipIndex::<T>::get(actor1, actor2)?;
            let rel2_id = RelationshipIndex::<T>::get(actor2, actor1)?;

            let rel1 = Relationships::<T>::get(rel1_id)?;
            let rel2 = Relationships::<T>::get(rel2_id)?;

            Some((rel1, rel2))
        }

        pub fn get_discovery_request(
            request_id: DiscoveryRequestId,
        ) -> Option<DiscoveryRequest<T>> {
            DiscoveryRequests::<T>::get(request_id)
        }

        pub fn get_pending_discovery_count() -> u32 {
            PendingDiscovery::<T>::get().len() as u32
        }

        pub fn can_discover(actor: ActorId) -> bool {
            let block_number = frame_system::Pallet::<T>::block_number();
            let last_discovery = LastDiscoveryBlock::<T>::get(actor);
            let rate_limit = T::DiscoveryRateLimitBlocks::get();

            if last_discovery == BlockNumberFor::<T>::default() {
                return true;
            }

            block_number >= last_discovery.saturating_add(rate_limit)
        }

        fn account_to_actor(account: &T::AccountId) -> ActorId {
            let encoded = account.encode();
            let mut bytes = [0u8; 32];
            let len = encoded.len().min(32);
            bytes[..len].copy_from_slice(&encoded[..len]);
            ActorId::from_raw(bytes)
        }

        fn update_profile_relationship_count(
            actor: ActorId,
            block_number: BlockNumberFor<T>,
            increment: bool,
        ) {
            let block_number_u64: u64 = block_number.try_into().unwrap_or(0u64);

            SemanticProfiles::<T>::mutate(actor, |profile| {
                if let Some(ref mut p) = profile {
                    if increment {
                        p.total_relationships = p.total_relationships.saturating_add(1);
                    } else {
                        p.total_relationships = p.total_relationships.saturating_sub(1);
                    }
                    p.last_activity_block = block_number_u64;
                } else {
                    *profile = Some(SemanticProfile {
                        actor,
                        total_relationships: if increment { 1 } else { 0 },
                        trust_score: 0,
                        discovery_enabled: true,
                        last_activity_block: block_number_u64,
                    });
                }
            });
        }
    }
}
