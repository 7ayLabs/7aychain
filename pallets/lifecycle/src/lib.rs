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
pub enum ActorStatus {
    Pending,
    Active,
    Suspended,
    Destroying,
    Destroyed,
}

impl Default for ActorStatus {
    fn default() -> Self {
        Self::Pending
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
pub enum KeyStatus {
    Active,
    Rotating,
    Destroying,
    Destroyed,
}

impl Default for KeyStatus {
    fn default() -> Self {
        Self::Active
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
pub enum DestructionReason {
    OwnerRequest,
    SecurityBreach,
    Expiration,
    ProtocolViolation,
    Administrative,
}

impl Default for DestructionReason {
    fn default() -> Self {
        Self::OwnerRequest
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
)]
#[scale_info(skip_type_params(T))]
pub struct ActorLifecycle<T: Config> {
    pub actor: ActorId,
    pub status: ActorStatus,
    pub created_at: BlockNumberFor<T>,
    pub last_active: BlockNumberFor<T>,
    pub key_hash: H256,
    pub key_status: KeyStatus,
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
#[scale_info(skip_type_params(T))]
pub struct KeyDestructionRequest<T: Config> {
    pub actor: ActorId,
    pub key_hash: H256,
    pub reason: DestructionReason,
    pub initiated_at: BlockNumberFor<T>,
    pub timeout_at: BlockNumberFor<T>,
    pub attestations: u32,
    pub finalized: bool,
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
#[scale_info(skip_type_params(T))]
pub struct DestructionAttestation<T: Config> {
    pub attester: ActorId,
    pub attested_at: BlockNumberFor<T>,
    pub signature_hash: H256,
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
#[scale_info(skip_type_params(T))]
pub struct KeyRotation<T: Config> {
    pub actor: ActorId,
    pub old_key_hash: H256,
    pub new_key_hash: H256,
    pub initiated_at: BlockNumberFor<T>,
    pub completed: bool,
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
        type KeyDestructionTimeoutBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MinDestructionAttestations: Get<u32>;

        #[pallet::constant]
        type RotationCooldownBlocks: Get<BlockNumberFor<Self>>;
    }

    #[pallet::storage]
    #[pallet::getter(fn actor_count)]
    pub type ActorCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn actors)]
    pub type Actors<T: Config> = StorageMap<_, Blake2_128Concat, ActorId, ActorLifecycle<T>>;

    #[pallet::storage]
    #[pallet::getter(fn destruction_requests)]
    pub type DestructionRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, ActorId, KeyDestructionRequest<T>>;

    #[pallet::storage]
    #[pallet::getter(fn destruction_attestations)]
    pub type DestructionAttestations<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ActorId,
        Blake2_128Concat,
        ActorId,
        DestructionAttestation<T>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn key_rotations)]
    pub type KeyRotations<T: Config> = StorageMap<_, Blake2_128Concat, ActorId, KeyRotation<T>>;

    #[pallet::storage]
    #[pallet::getter(fn destroyed_keys)]
    pub type DestroyedKeys<T: Config> = StorageMap<_, Blake2_128Concat, H256, BlockNumberFor<T>>;

    #[pallet::storage]
    #[pallet::getter(fn active_actors)]
    pub type ActiveActors<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            ActorCount::<T>::put(0u64);
            ActiveActors::<T>::put(0u64);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            Self::process_destruction_timeouts(n)
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ActorRegistered {
            actor: ActorId,
            key_hash: H256,
        },
        ActorActivated {
            actor: ActorId,
        },
        ActorSuspended {
            actor: ActorId,
        },
        ActorReactivated {
            actor: ActorId,
        },
        KeyDestructionInitiated {
            actor: ActorId,
            reason: DestructionReason,
            timeout_at: BlockNumberFor<T>,
        },
        DestructionAttested {
            actor: ActorId,
            attester: ActorId,
            attestation_count: u32,
        },
        KeyDestroyed {
            actor: ActorId,
            key_hash: H256,
        },
        KeyRotationInitiated {
            actor: ActorId,
            old_key_hash: H256,
            new_key_hash: H256,
        },
        KeyRotationCompleted {
            actor: ActorId,
            new_key_hash: H256,
        },
        DestructionCancelled {
            actor: ActorId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        ActorNotFound,
        ActorAlreadyExists,
        ActorNotActive,
        ActorSuspended,
        ActorDestroyed,
        DestructionNotPending,
        DestructionAlreadyPending,
        AlreadyAttested,
        InsufficientAttestations,
        DestructionTimedOut,
        KeyRotationPending,
        RotationCooldownActive,
        InvalidKeyHash,
        NotAuthorized,
        CannotSelfAttest,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_actor())]
        pub fn register_actor(origin: OriginFor<T>, key_hash: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(who);

            ensure!(
                !Actors::<T>::contains_key(actor),
                Error::<T>::ActorAlreadyExists
            );

            let block_number = frame_system::Pallet::<T>::block_number();

            let lifecycle = ActorLifecycle {
                actor,
                status: ActorStatus::Pending,
                created_at: block_number,
                last_active: block_number,
                key_hash,
                key_status: KeyStatus::Active,
            };

            Actors::<T>::insert(actor, lifecycle);
            ActorCount::<T>::mutate(|c| *c = c.saturating_add(1));

            Self::deposit_event(Event::ActorRegistered { actor, key_hash });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::activate_actor())]
        pub fn activate_actor(origin: OriginFor<T>, actor: ActorId) -> DispatchResult {
            ensure_root(origin)?;

            Actors::<T>::try_mutate(actor, |lifecycle| -> DispatchResult {
                let l = lifecycle.as_mut().ok_or(Error::<T>::ActorNotFound)?;

                ensure!(l.status == ActorStatus::Pending, Error::<T>::ActorNotActive);

                l.status = ActorStatus::Active;
                l.last_active = frame_system::Pallet::<T>::block_number();

                ActiveActors::<T>::mutate(|c| *c = c.saturating_add(1));

                Self::deposit_event(Event::ActorActivated { actor });

                Ok(())
            })
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::suspend_actor())]
        pub fn suspend_actor(origin: OriginFor<T>, actor: ActorId) -> DispatchResult {
            ensure_root(origin)?;

            Actors::<T>::try_mutate(actor, |lifecycle| -> DispatchResult {
                let l = lifecycle.as_mut().ok_or(Error::<T>::ActorNotFound)?;

                ensure!(l.status == ActorStatus::Active, Error::<T>::ActorNotActive);

                l.status = ActorStatus::Suspended;
                l.last_active = frame_system::Pallet::<T>::block_number();

                ActiveActors::<T>::mutate(|c| *c = c.saturating_sub(1));

                Self::deposit_event(Event::ActorSuspended { actor });

                Ok(())
            })
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::reactivate_actor())]
        pub fn reactivate_actor(origin: OriginFor<T>, actor: ActorId) -> DispatchResult {
            ensure_root(origin)?;

            Actors::<T>::try_mutate(actor, |lifecycle| -> DispatchResult {
                let l = lifecycle.as_mut().ok_or(Error::<T>::ActorNotFound)?;

                ensure!(
                    l.status == ActorStatus::Suspended,
                    Error::<T>::ActorSuspended
                );

                l.status = ActorStatus::Active;
                l.last_active = frame_system::Pallet::<T>::block_number();

                ActiveActors::<T>::mutate(|c| *c = c.saturating_add(1));

                Self::deposit_event(Event::ActorReactivated { actor });

                Ok(())
            })
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::initiate_destruction())]
        pub fn initiate_destruction(
            origin: OriginFor<T>,
            reason: DestructionReason,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(who);

            let lifecycle = Actors::<T>::get(actor).ok_or(Error::<T>::ActorNotFound)?;

            ensure!(
                lifecycle.status == ActorStatus::Active
                    || lifecycle.status == ActorStatus::Suspended,
                Error::<T>::ActorDestroyed
            );
            ensure!(
                !DestructionRequests::<T>::contains_key(actor),
                Error::<T>::DestructionAlreadyPending
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            let timeout_at = block_number.saturating_add(T::KeyDestructionTimeoutBlocks::get());

            let request = KeyDestructionRequest {
                actor,
                key_hash: lifecycle.key_hash,
                reason,
                initiated_at: block_number,
                timeout_at,
                attestations: 0,
                finalized: false,
            };

            DestructionRequests::<T>::insert(actor, request);

            Actors::<T>::mutate(actor, |l| {
                if let Some(ref mut lifecycle) = l {
                    lifecycle.status = ActorStatus::Destroying;
                    lifecycle.key_status = KeyStatus::Destroying;
                }
            });

            Self::deposit_event(Event::KeyDestructionInitiated {
                actor,
                reason,
                timeout_at,
            });

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::attest_destruction())]
        pub fn attest_destruction(
            origin: OriginFor<T>,
            target_actor: ActorId,
            signature_hash: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let attester = Self::account_to_actor(who);

            ensure!(attester != target_actor, Error::<T>::CannotSelfAttest);

            let attester_lifecycle = Actors::<T>::get(attester).ok_or(Error::<T>::ActorNotFound)?;
            ensure!(
                attester_lifecycle.status == ActorStatus::Active,
                Error::<T>::ActorNotActive
            );

            ensure!(
                DestructionRequests::<T>::contains_key(target_actor),
                Error::<T>::DestructionNotPending
            );
            ensure!(
                !DestructionAttestations::<T>::contains_key(target_actor, attester),
                Error::<T>::AlreadyAttested
            );

            let block_number = frame_system::Pallet::<T>::block_number();

            let attestation = DestructionAttestation {
                attester,
                attested_at: block_number,
                signature_hash,
            };

            DestructionAttestations::<T>::insert(target_actor, attester, attestation);

            DestructionRequests::<T>::mutate(target_actor, |request| {
                if let Some(ref mut r) = request {
                    r.attestations = r.attestations.saturating_add(1);
                }
            });

            let attestation_count = DestructionRequests::<T>::get(target_actor)
                .map(|r| r.attestations)
                .unwrap_or(0);

            Self::deposit_event(Event::DestructionAttested {
                actor: target_actor,
                attester,
                attestation_count,
            });

            if attestation_count >= T::MinDestructionAttestations::get() {
                Self::finalize_destruction(target_actor)?;
            }

            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::cancel_destruction())]
        pub fn cancel_destruction(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(who);

            let request =
                DestructionRequests::<T>::get(actor).ok_or(Error::<T>::DestructionNotPending)?;

            ensure!(!request.finalized, Error::<T>::ActorDestroyed);

            DestructionRequests::<T>::remove(actor);

            let _ = DestructionAttestations::<T>::clear_prefix(actor, u32::MAX, None);

            Actors::<T>::mutate(actor, |l| {
                if let Some(ref mut lifecycle) = l {
                    lifecycle.status = ActorStatus::Active;
                    lifecycle.key_status = KeyStatus::Active;
                }
            });

            Self::deposit_event(Event::DestructionCancelled { actor });

            Ok(())
        }

        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::initiate_rotation())]
        pub fn initiate_rotation(origin: OriginFor<T>, new_key_hash: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(who);

            let lifecycle = Actors::<T>::get(actor).ok_or(Error::<T>::ActorNotFound)?;

            ensure!(
                lifecycle.status == ActorStatus::Active,
                Error::<T>::ActorNotActive
            );
            ensure!(
                lifecycle.key_status == KeyStatus::Active,
                Error::<T>::KeyRotationPending
            );
            ensure!(
                !KeyRotations::<T>::contains_key(actor),
                Error::<T>::KeyRotationPending
            );

            let block_number = frame_system::Pallet::<T>::block_number();

            let rotation = KeyRotation {
                actor,
                old_key_hash: lifecycle.key_hash,
                new_key_hash,
                initiated_at: block_number,
                completed: false,
            };

            KeyRotations::<T>::insert(actor, rotation.clone());

            Actors::<T>::mutate(actor, |l| {
                if let Some(ref mut lc) = l {
                    lc.key_status = KeyStatus::Rotating;
                }
            });

            Self::deposit_event(Event::KeyRotationInitiated {
                actor,
                old_key_hash: lifecycle.key_hash,
                new_key_hash,
            });

            Ok(())
        }

        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::complete_rotation())]
        pub fn complete_rotation(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(who);

            let rotation = KeyRotations::<T>::get(actor).ok_or(Error::<T>::KeyRotationPending)?;

            ensure!(!rotation.completed, Error::<T>::KeyRotationPending);

            let block_number = frame_system::Pallet::<T>::block_number();

            KeyRotations::<T>::mutate(actor, |r| {
                if let Some(ref mut rot) = r {
                    rot.completed = true;
                }
            });

            Actors::<T>::mutate(actor, |l| {
                if let Some(ref mut lifecycle) = l {
                    lifecycle.key_hash = rotation.new_key_hash;
                    lifecycle.key_status = KeyStatus::Active;
                    lifecycle.last_active = block_number;
                }
            });

            Self::deposit_event(Event::KeyRotationCompleted {
                actor,
                new_key_hash: rotation.new_key_hash,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn account_to_actor(account: T::AccountId) -> ActorId {
            let encoded = account.encode();
            let hash = sp_core::blake2_256(&encoded);
            ActorId::from_raw(hash)
        }

        fn finalize_destruction(actor: ActorId) -> DispatchResult {
            let lifecycle = Actors::<T>::get(actor).ok_or(Error::<T>::ActorNotFound)?;
            let key_hash = lifecycle.key_hash;

            DestructionRequests::<T>::mutate(actor, |request| {
                if let Some(ref mut r) = request {
                    r.finalized = true;
                }
            });

            Actors::<T>::mutate(actor, |l| {
                if let Some(ref mut lc) = l {
                    lc.status = ActorStatus::Destroyed;
                    lc.key_status = KeyStatus::Destroyed;
                }
            });

            let block_number = frame_system::Pallet::<T>::block_number();
            DestroyedKeys::<T>::insert(key_hash, block_number);

            ActiveActors::<T>::mutate(|c| *c = c.saturating_sub(1));

            Self::deposit_event(Event::KeyDestroyed { actor, key_hash });

            Ok(())
        }

        #[allow(clippy::excessive_nesting)]
        fn process_destruction_timeouts(current_block: BlockNumberFor<T>) -> Weight {
            let mut processed = 0u32;
            let max_process = 5u32;

            for (actor, request) in DestructionRequests::<T>::iter() {
                if processed >= max_process {
                    break;
                }

                if current_block >= request.timeout_at && !request.finalized {
                    if request.attestations >= T::MinDestructionAttestations::get() {
                        let _ = Self::finalize_destruction(actor);
                    }
                    processed = processed.saturating_add(1);
                }
            }

            Weight::from_parts(processed as u64 * 15_000, 0)
        }

        pub fn is_actor_active(actor: ActorId) -> bool {
            Actors::<T>::get(actor)
                .map(|l| l.status == ActorStatus::Active)
                .unwrap_or(false)
        }

        pub fn is_key_valid(actor: ActorId, key_hash: &H256) -> bool {
            Actors::<T>::get(actor)
                .map(|l| l.key_hash == *key_hash && l.key_status == KeyStatus::Active)
                .unwrap_or(false)
        }

        pub fn is_destruction_pending(actor: ActorId) -> bool {
            DestructionRequests::<T>::get(actor)
                .map(|r| !r.finalized)
                .unwrap_or(false)
        }

        pub fn get_attestation_count(actor: ActorId) -> u32 {
            DestructionRequests::<T>::get(actor)
                .map(|r| r.attestations)
                .unwrap_or(0)
        }

        pub fn total_actors() -> u64 {
            ActorCount::<T>::get()
        }

        pub fn total_active() -> u64 {
            ActiveActors::<T>::get()
        }
    }
}
