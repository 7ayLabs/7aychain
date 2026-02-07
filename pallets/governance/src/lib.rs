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
    use alloc::vec;
    use alloc::vec::Vec;

    use crate::WeightInfo;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct CapabilityId(pub u64);

    impl CapabilityId {
        pub const fn new(id: u64) -> Self {
            Self(id)
        }

        pub const fn inner(self) -> u64 {
            self.0
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct ResourceId(pub [u8; 32]);

    impl ResourceId {
        pub const fn from_bytes(bytes: [u8; 32]) -> Self {
            Self(bytes)
        }

        pub fn as_bytes(&self) -> &[u8; 32] {
            &self.0
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub struct Permissions(pub u32);

    impl Permissions {
        pub const NONE: Self = Self(0);
        pub const READ: Self = Self(1 << 0);
        pub const WRITE: Self = Self(1 << 1);
        pub const EXECUTE: Self = Self(1 << 2);
        pub const DELEGATE: Self = Self(1 << 3);
        pub const ADMIN: Self = Self(1 << 4);
        pub const ALL: Self = Self(0b11111);

        pub const fn new(bits: u32) -> Self {
            Self(bits)
        }

        pub const fn contains(self, other: Self) -> bool {
            (self.0 & other.0) == other.0
        }

        pub const fn union(self, other: Self) -> Self {
            Self(self.0 | other.0)
        }

        pub const fn intersection(self, other: Self) -> Self {
            Self(self.0 & other.0)
        }

        pub const fn is_empty(self) -> bool {
            self.0 == 0
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    pub enum CapabilityStatus {
        Active,
        Revoked,
        Expired,
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct Capability<T: Config> {
        pub id: CapabilityId,
        pub grantor: T::AccountId,
        pub grantee: ActorId,
        pub resource: ResourceId,
        pub permissions: Permissions,
        pub status: CapabilityStatus,
        pub created_at: BlockNumberFor<T>,
        pub expires_at: Option<BlockNumberFor<T>>,
        pub delegatable: bool,
        pub parent_capability: Option<CapabilityId>,
    }

    #[derive(Clone, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, MaxEncodedLen, TypeInfo, RuntimeDebug)]
    #[scale_info(skip_type_params(T))]
    pub struct DelegationRecord<T: Config> {
        pub original_capability: CapabilityId,
        pub delegated_capability: CapabilityId,
        pub delegator: ActorId,
        pub delegated_at: BlockNumberFor<T>,
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type WeightInfo: WeightInfo;

        #[pallet::constant]
        type MaxCapabilitiesPerActor: Get<u32>;

        #[pallet::constant]
        type MaxDelegationDepth: Get<u32>;

        #[pallet::constant]
        type DefaultCapabilityDuration: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MaxCapabilitiesPerResource: Get<u32>;
    }

    #[pallet::storage]
    #[pallet::getter(fn capabilities)]
    pub type Capabilities<T: Config> =
        StorageMap<_, Blake2_128Concat, CapabilityId, Capability<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn capability_count)]
    pub type CapabilityCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn actor_capabilities)]
    pub type ActorCapabilities<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ActorId,
        BoundedVec<CapabilityId, T::MaxCapabilitiesPerActor>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn resource_capabilities)]
    pub type ResourceCapabilities<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ResourceId,
        BoundedVec<CapabilityId, T::MaxCapabilitiesPerResource>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn delegations)]
    pub type Delegations<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        CapabilityId,
        Blake2_128Concat,
        CapabilityId,
        DelegationRecord<T>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn delegation_depth)]
    pub type DelegationDepth<T: Config> =
        StorageMap<_, Blake2_128Concat, CapabilityId, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        CapabilityGranted {
            capability_id: CapabilityId,
            grantor: T::AccountId,
            grantee: ActorId,
            resource: ResourceId,
            permissions: Permissions,
        },
        CapabilityRevoked {
            capability_id: CapabilityId,
            revoker: T::AccountId,
        },
        CapabilityDelegated {
            original_capability: CapabilityId,
            delegated_capability: CapabilityId,
            delegator: ActorId,
            delegatee: ActorId,
        },
        CapabilityExpired {
            capability_id: CapabilityId,
        },
        CapabilityUpdated {
            capability_id: CapabilityId,
            new_permissions: Permissions,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        CapabilityNotFound,
        CapabilityExpired,
        CapabilityRevoked,
        InsufficientPermissions,
        NotAuthorized,
        MaxCapabilitiesReached,
        MaxDelegationDepthReached,
        CapabilityNotDelegatable,
        InvalidPermissions,
        ResourceNotFound,
        DelegationNotFound,
        SelfDelegation,
        CannotRevokeParentCapability,
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
            CapabilityCount::<T>::put(0u64);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut expired_count = 0u32;

            for (id, capability) in Capabilities::<T>::iter() {
                if capability.status == CapabilityStatus::Active {
                    if let Some(expires_at) = capability.expires_at {
                        if now >= expires_at {
                            Capabilities::<T>::mutate(id, |cap| {
                                if let Some(ref mut c) = cap {
                                    c.status = CapabilityStatus::Expired;
                                }
                            });
                            Self::deposit_event(Event::CapabilityExpired { capability_id: id });
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
        #[pallet::weight(T::WeightInfo::grant_capability())]
        pub fn grant_capability(
            origin: OriginFor<T>,
            grantee: ActorId,
            resource: ResourceId,
            permissions: Permissions,
            expires_at: Option<BlockNumberFor<T>>,
            delegatable: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            ensure!(!permissions.is_empty(), Error::<T>::InvalidPermissions);

            let capability_id = CapabilityId::new(CapabilityCount::<T>::get());
            CapabilityCount::<T>::put(capability_id.inner().saturating_add(1));

            let capability = Capability {
                id: capability_id,
                grantor: who.clone(),
                grantee,
                resource,
                permissions,
                status: CapabilityStatus::Active,
                created_at: block_number,
                expires_at,
                delegatable,
                parent_capability: None,
            };

            Capabilities::<T>::insert(capability_id, capability);

            ActorCapabilities::<T>::try_mutate(grantee, |caps| {
                caps.try_push(capability_id)
                    .map_err(|_| Error::<T>::MaxCapabilitiesReached)
            })?;

            ResourceCapabilities::<T>::try_mutate(resource, |caps| {
                caps.try_push(capability_id)
                    .map_err(|_| Error::<T>::MaxCapabilitiesReached)
            })?;

            Self::deposit_event(Event::CapabilityGranted {
                capability_id,
                grantor: who,
                grantee,
                resource,
                permissions,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::revoke_capability())]
        pub fn revoke_capability(
            origin: OriginFor<T>,
            capability_id: CapabilityId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut capability =
                Capabilities::<T>::get(capability_id).ok_or(Error::<T>::CapabilityNotFound)?;

            ensure!(capability.grantor == who, Error::<T>::NotAuthorized);
            ensure!(
                capability.status == CapabilityStatus::Active,
                Error::<T>::CapabilityRevoked
            );

            capability.status = CapabilityStatus::Revoked;
            Capabilities::<T>::insert(capability_id, capability.clone());

            Self::revoke_delegated_capabilities(capability_id, block_number);

            Self::deposit_event(Event::CapabilityRevoked {
                capability_id,
                revoker: who,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::delegate_capability())]
        pub fn delegate_capability(
            origin: OriginFor<T>,
            capability_id: CapabilityId,
            delegatee: ActorId,
            permissions: Permissions,
            expires_at: Option<BlockNumberFor<T>>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let capability =
                Capabilities::<T>::get(capability_id).ok_or(Error::<T>::CapabilityNotFound)?;

            ensure!(
                capability.status == CapabilityStatus::Active,
                Error::<T>::CapabilityRevoked
            );
            ensure!(capability.delegatable, Error::<T>::CapabilityNotDelegatable);
            ensure!(capability.grantee != delegatee, Error::<T>::SelfDelegation);

            let actor_id = Self::account_to_actor(&who);
            ensure!(capability.grantee == actor_id, Error::<T>::NotAuthorized);

            ensure!(
                capability.permissions.contains(permissions),
                Error::<T>::InsufficientPermissions
            );

            let current_depth = DelegationDepth::<T>::get(capability_id);
            ensure!(
                current_depth < T::MaxDelegationDepth::get(),
                Error::<T>::MaxDelegationDepthReached
            );

            let delegated_expiry = match (capability.expires_at, expires_at) {
                (Some(cap_exp), Some(req_exp)) => Some(cap_exp.min(req_exp)),
                (Some(cap_exp), None) => Some(cap_exp),
                (None, Some(req_exp)) => Some(req_exp),
                (None, None) => None,
            };

            let new_capability_id = CapabilityId::new(CapabilityCount::<T>::get());
            CapabilityCount::<T>::put(new_capability_id.inner().saturating_add(1));

            let new_capability = Capability {
                id: new_capability_id,
                grantor: who,
                grantee: delegatee,
                resource: capability.resource,
                permissions,
                status: CapabilityStatus::Active,
                created_at: block_number,
                expires_at: delegated_expiry,
                delegatable: capability.delegatable,
                parent_capability: Some(capability_id),
            };

            Capabilities::<T>::insert(new_capability_id, new_capability);

            DelegationDepth::<T>::insert(new_capability_id, current_depth.saturating_add(1));

            let delegation_record = DelegationRecord {
                original_capability: capability_id,
                delegated_capability: new_capability_id,
                delegator: actor_id,
                delegated_at: block_number,
            };

            Delegations::<T>::insert(capability_id, new_capability_id, delegation_record);

            ActorCapabilities::<T>::try_mutate(delegatee, |caps| {
                caps.try_push(new_capability_id)
                    .map_err(|_| Error::<T>::MaxCapabilitiesReached)
            })?;

            ResourceCapabilities::<T>::try_mutate(capability.resource, |caps| {
                caps.try_push(new_capability_id)
                    .map_err(|_| Error::<T>::MaxCapabilitiesReached)
            })?;

            Self::deposit_event(Event::CapabilityDelegated {
                original_capability: capability_id,
                delegated_capability: new_capability_id,
                delegator: actor_id,
                delegatee,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::update_capability())]
        pub fn update_capability(
            origin: OriginFor<T>,
            capability_id: CapabilityId,
            new_permissions: Permissions,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mut capability =
                Capabilities::<T>::get(capability_id).ok_or(Error::<T>::CapabilityNotFound)?;

            ensure!(capability.grantor == who, Error::<T>::NotAuthorized);
            ensure!(
                capability.status == CapabilityStatus::Active,
                Error::<T>::CapabilityRevoked
            );
            ensure!(!new_permissions.is_empty(), Error::<T>::InvalidPermissions);

            capability.permissions = new_permissions;
            Capabilities::<T>::insert(capability_id, capability);

            Self::deposit_event(Event::CapabilityUpdated {
                capability_id,
                new_permissions,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn get_capability(capability_id: CapabilityId) -> Option<Capability<T>> {
            Capabilities::<T>::get(capability_id)
        }

        pub fn has_permission(
            actor: ActorId,
            resource: ResourceId,
            required: Permissions,
        ) -> bool {
            let block_number = frame_system::Pallet::<T>::block_number();

            ActorCapabilities::<T>::get(actor)
                .iter()
                .filter_map(|cap_id| Capabilities::<T>::get(cap_id))
                .any(|cap| {
                    cap.resource == resource
                        && cap.status == CapabilityStatus::Active
                        && cap.permissions.contains(required)
                        && cap.expires_at.map_or(true, |exp| block_number < exp)
                })
        }

        pub fn get_actor_capabilities(actor: ActorId) -> Vec<Capability<T>> {
            ActorCapabilities::<T>::get(actor)
                .iter()
                .filter_map(|cap_id| Capabilities::<T>::get(cap_id))
                .collect()
        }

        pub fn get_resource_capabilities(resource: ResourceId) -> Vec<Capability<T>> {
            ResourceCapabilities::<T>::get(resource)
                .iter()
                .filter_map(|cap_id| Capabilities::<T>::get(cap_id))
                .collect()
        }

        pub fn is_capability_active(capability_id: CapabilityId) -> bool {
            let block_number = frame_system::Pallet::<T>::block_number();

            Capabilities::<T>::get(capability_id)
                .map(|cap| {
                    cap.status == CapabilityStatus::Active
                        && cap.expires_at.map_or(true, |exp| block_number < exp)
                })
                .unwrap_or(false)
        }

        pub fn get_delegation_chain(capability_id: CapabilityId) -> Vec<CapabilityId> {
            let mut chain = vec![capability_id];
            let mut current = capability_id;

            while let Some(cap) = Capabilities::<T>::get(current) {
                if let Some(parent) = cap.parent_capability {
                    chain.push(parent);
                    current = parent;
                } else {
                    break;
                }
            }

            chain.reverse();
            chain
        }

        fn revoke_delegated_capabilities(
            parent_capability_id: CapabilityId,
            _block_number: BlockNumberFor<T>,
        ) {
            for (delegated_id, _) in Delegations::<T>::iter_prefix(parent_capability_id) {
                Capabilities::<T>::mutate(delegated_id, |cap| {
                    if let Some(ref mut c) = cap {
                        if c.status == CapabilityStatus::Active {
                            c.status = CapabilityStatus::Revoked;
                        }
                    }
                });

                Self::revoke_delegated_capabilities(delegated_id, _block_number);
            }
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
