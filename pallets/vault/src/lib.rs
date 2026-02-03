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
use sp_runtime::Saturating;
use alloc::vec::Vec;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Default, Hash,
)]
pub struct VaultId(pub u64);

impl VaultId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Default, Hash,
)]
pub struct ShareId(pub u64);

impl ShareId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub enum VaultStatus {
    Creating,
    Active,
    Locked,
    Recovering,
    Dissolved,
}

impl Default for VaultStatus {
    fn default() -> Self {
        Self::Creating
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub enum MemberRole {
    Owner,
    Guardian,
    Participant,
}

impl Default for MemberRole {
    fn default() -> Self {
        Self::Participant
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub enum ShareStatus {
    Pending,
    Distributed,
    Revealed,
    Invalidated,
}

impl Default for ShareStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Vault<T: Config> {
    pub id: VaultId,
    pub owner: ActorId,
    pub status: VaultStatus,
    pub threshold: u32,
    pub ring_size: u32,
    pub member_count: u32,
    pub secret_hash: H256,
    pub created_at: BlockNumberFor<T>,
    pub last_activity: BlockNumberFor<T>,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct VaultMember<T: Config> {
    pub vault: VaultId,
    pub actor: ActorId,
    pub role: MemberRole,
    pub share_index: u32,
    pub joined_at: BlockNumberFor<T>,
    pub share_committed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Share<T: Config> {
    pub id: ShareId,
    pub vault: VaultId,
    pub holder: ActorId,
    pub index: u32,
    pub commitment: H256,
    pub status: ShareStatus,
    pub created_at: BlockNumberFor<T>,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct RecoveryRequest<T: Config> {
    pub vault: VaultId,
    pub requester: ActorId,
    pub shares_revealed: u32,
    pub initiated_at: BlockNumberFor<T>,
    pub expires_at: BlockNumberFor<T>,
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
        type MinThreshold: Get<u32>;

        #[pallet::constant]
        type MinRingSize: Get<u32>;

        #[pallet::constant]
        type MaxRingSize: Get<u32>;

        #[pallet::constant]
        type RecoveryPeriodBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MaxVaultsPerActor: Get<u32>;
    }

    #[pallet::storage]
    #[pallet::getter(fn vault_count)]
    pub type VaultCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn share_count)]
    pub type ShareCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn vaults)]
    pub type Vaults<T: Config> = StorageMap<_, Blake2_128Concat, VaultId, Vault<T>>;

    #[pallet::storage]
    #[pallet::getter(fn vault_members)]
    pub type VaultMembers<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, VaultId, Blake2_128Concat, ActorId, VaultMember<T>>;

    #[pallet::storage]
    #[pallet::getter(fn shares)]
    pub type Shares<T: Config> = StorageMap<_, Blake2_128Concat, ShareId, Share<T>>;

    #[pallet::storage]
    #[pallet::getter(fn actor_shares)]
    pub type ActorShares<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, ActorId, Blake2_128Concat, ShareId, ()>;

    #[pallet::storage]
    #[pallet::getter(fn vault_shares)]
    pub type VaultShares<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, VaultId, Blake2_128Concat, ShareId, ()>;

    #[pallet::storage]
    #[pallet::getter(fn recovery_requests)]
    pub type RecoveryRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, VaultId, RecoveryRequest<T>>;

    #[pallet::storage]
    #[pallet::getter(fn actor_vaults)]
    pub type ActorVaults<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, ActorId, Blake2_128Concat, VaultId, ()>;

    #[pallet::storage]
    #[pallet::getter(fn vault_count_per_actor)]
    pub type VaultCountPerActor<T: Config> =
        StorageMap<_, Blake2_128Concat, ActorId, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn active_vault_count)]
    pub type ActiveVaultCount<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            VaultCount::<T>::put(0u64);
            ShareCount::<T>::put(0u64);
            ActiveVaultCount::<T>::put(0u32);
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        VaultCreated {
            vault_id: VaultId,
            owner: ActorId,
            threshold: u32,
            ring_size: u32,
        },
        MemberAdded {
            vault_id: VaultId,
            member: ActorId,
            role: MemberRole,
        },
        VaultActivated {
            vault_id: VaultId,
        },
        ShareCommitted {
            vault_id: VaultId,
            share_id: ShareId,
            holder: ActorId,
        },
        ShareRevealed {
            vault_id: VaultId,
            share_id: ShareId,
        },
        RecoveryInitiated {
            vault_id: VaultId,
            requester: ActorId,
        },
        RecoveryCompleted {
            vault_id: VaultId,
        },
        VaultLocked {
            vault_id: VaultId,
        },
        VaultDissolved {
            vault_id: VaultId,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        VaultNotFound,
        MemberNotFound,
        ShareNotFound,
        InvalidThreshold,
        InvalidRingSize,
        ThresholdExceedsRingSize,
        MaxVaultsReached,
        MemberAlreadyExists,
        NotVaultOwner,
        NotVaultMember,
        VaultNotActive,
        VaultAlreadyActive,
        ShareAlreadyCommitted,
        ShareNotDistributed,
        RecoveryAlreadyActive,
        RecoveryNotActive,
        InsufficientShares,
        RecoveryExpired,
        VaultLocked,
        CannotDissolvActiveVault,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_vault())]
        pub fn create_vault(
            origin: OriginFor<T>,
            owner: ActorId,
            threshold: u32,
            ring_size: u32,
            secret_hash: H256,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            ensure!(
                threshold >= T::MinThreshold::get(),
                Error::<T>::InvalidThreshold
            );
            ensure!(
                ring_size >= T::MinRingSize::get() && ring_size <= T::MaxRingSize::get(),
                Error::<T>::InvalidRingSize
            );
            ensure!(
                threshold <= ring_size,
                Error::<T>::ThresholdExceedsRingSize
            );

            let vault_count = VaultCountPerActor::<T>::get(owner);
            ensure!(
                vault_count < T::MaxVaultsPerActor::get(),
                Error::<T>::MaxVaultsReached
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            let vault_id = Self::next_vault_id();

            let vault = Vault {
                id: vault_id,
                owner,
                status: VaultStatus::Creating,
                threshold,
                ring_size,
                member_count: 1,
                secret_hash,
                created_at: block_number,
                last_activity: block_number,
            };

            let owner_member = VaultMember {
                vault: vault_id,
                actor: owner,
                role: MemberRole::Owner,
                share_index: 0,
                joined_at: block_number,
                share_committed: false,
            };

            Vaults::<T>::insert(vault_id, vault);
            VaultMembers::<T>::insert(vault_id, owner, owner_member);
            ActorVaults::<T>::insert(owner, vault_id, ());
            VaultCountPerActor::<T>::mutate(owner, |count| *count = count.saturating_add(1));

            Self::deposit_event(Event::VaultCreated {
                vault_id,
                owner,
                threshold,
                ring_size,
            });

            Self::deposit_event(Event::MemberAdded {
                vault_id,
                member: owner,
                role: MemberRole::Owner,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::add_member())]
        pub fn add_member(
            origin: OriginFor<T>,
            vault_id: VaultId,
            member: ActorId,
            role: MemberRole,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let mut vault = Vaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;

            ensure!(
                vault.status == VaultStatus::Creating,
                Error::<T>::VaultAlreadyActive
            );
            ensure!(
                !VaultMembers::<T>::contains_key(vault_id, member),
                Error::<T>::MemberAlreadyExists
            );
            ensure!(
                vault.member_count < vault.ring_size,
                Error::<T>::InvalidRingSize
            );

            let block_number = frame_system::Pallet::<T>::block_number();

            let vault_member = VaultMember {
                vault: vault_id,
                actor: member,
                role,
                share_index: vault.member_count,
                joined_at: block_number,
                share_committed: false,
            };

            vault.member_count = vault.member_count.saturating_add(1);
            vault.last_activity = block_number;

            Vaults::<T>::insert(vault_id, vault);
            VaultMembers::<T>::insert(vault_id, member, vault_member);
            ActorVaults::<T>::insert(member, vault_id, ());

            Self::deposit_event(Event::MemberAdded {
                vault_id,
                member,
                role,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::activate_vault())]
        pub fn activate_vault(origin: OriginFor<T>, vault_id: VaultId) -> DispatchResult {
            ensure_signed(origin)?;

            Vaults::<T>::try_mutate(vault_id, |vault| -> DispatchResult {
                let v = vault.as_mut().ok_or(Error::<T>::VaultNotFound)?;

                ensure!(
                    v.status == VaultStatus::Creating,
                    Error::<T>::VaultAlreadyActive
                );
                ensure!(
                    v.member_count >= v.ring_size,
                    Error::<T>::InvalidRingSize
                );

                v.status = VaultStatus::Active;
                v.last_activity = frame_system::Pallet::<T>::block_number();

                ActiveVaultCount::<T>::mutate(|count| *count = count.saturating_add(1));

                Self::deposit_event(Event::VaultActivated { vault_id });

                Ok(())
            })
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::commit_share())]
        pub fn commit_share(
            origin: OriginFor<T>,
            vault_id: VaultId,
            commitment: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let vault = Vaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;

            ensure!(
                vault.status == VaultStatus::Active,
                Error::<T>::VaultNotActive
            );

            let actor = Self::account_to_actor(who);

            VaultMembers::<T>::try_mutate(vault_id, actor, |member| -> DispatchResult {
                let m = member.as_mut().ok_or(Error::<T>::NotVaultMember)?;

                ensure!(!m.share_committed, Error::<T>::ShareAlreadyCommitted);

                m.share_committed = true;

                let block_number = frame_system::Pallet::<T>::block_number();
                let share_id = Self::next_share_id();

                let share = Share {
                    id: share_id,
                    vault: vault_id,
                    holder: actor,
                    index: m.share_index,
                    commitment,
                    status: ShareStatus::Distributed,
                    created_at: block_number,
                };

                Shares::<T>::insert(share_id, share);
                ActorShares::<T>::insert(actor, share_id, ());
                VaultShares::<T>::insert(vault_id, share_id, ());

                Self::deposit_event(Event::ShareCommitted {
                    vault_id,
                    share_id,
                    holder: actor,
                });

                Ok(())
            })?;

            Vaults::<T>::mutate(vault_id, |v| {
                if let Some(ref mut vault) = v {
                    vault.last_activity = frame_system::Pallet::<T>::block_number();
                }
            });

            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::initiate_recovery())]
        pub fn initiate_recovery(origin: OriginFor<T>, vault_id: VaultId) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let vault = Vaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;
            let actor = Self::account_to_actor(who);

            ensure!(
                !RecoveryRequests::<T>::contains_key(vault_id),
                Error::<T>::RecoveryAlreadyActive
            );
            ensure!(
                vault.status == VaultStatus::Active,
                Error::<T>::VaultNotActive
            );
            ensure!(
                VaultMembers::<T>::contains_key(vault_id, actor),
                Error::<T>::NotVaultMember
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            let expires_at = block_number.saturating_add(T::RecoveryPeriodBlocks::get());

            let request = RecoveryRequest {
                vault: vault_id,
                requester: actor,
                shares_revealed: 0,
                initiated_at: block_number,
                expires_at,
            };

            RecoveryRequests::<T>::insert(vault_id, request);

            Vaults::<T>::mutate(vault_id, |v| {
                if let Some(ref mut vault) = v {
                    vault.status = VaultStatus::Recovering;
                    vault.last_activity = block_number;
                }
            });

            Self::deposit_event(Event::RecoveryInitiated {
                vault_id,
                requester: actor,
            });

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::reveal_share())]
        pub fn reveal_share(origin: OriginFor<T>, share_id: ShareId) -> DispatchResult {
            ensure_signed(origin)?;

            let share = Shares::<T>::get(share_id).ok_or(Error::<T>::ShareNotFound)?;
            let vault_id = share.vault;

            let vault = Vaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;

            ensure!(
                vault.status == VaultStatus::Recovering,
                Error::<T>::RecoveryNotActive
            );
            ensure!(
                share.status == ShareStatus::Distributed,
                Error::<T>::ShareNotDistributed
            );

            let block_number = frame_system::Pallet::<T>::block_number();

            Shares::<T>::mutate(share_id, |s| {
                if let Some(ref mut sh) = s {
                    sh.status = ShareStatus::Revealed;
                }
            });

            RecoveryRequests::<T>::try_mutate(vault_id, |request| -> DispatchResult {
                let r = request.as_mut().ok_or(Error::<T>::RecoveryNotActive)?;

                ensure!(block_number <= r.expires_at, Error::<T>::RecoveryExpired);

                r.shares_revealed = r.shares_revealed.saturating_add(1);

                if r.shares_revealed >= vault.threshold {
                    Vaults::<T>::mutate(vault_id, |v| {
                        if let Some(ref mut vault) = v {
                            vault.status = VaultStatus::Active;
                            vault.last_activity = block_number;
                        }
                    });

                    Self::deposit_event(Event::RecoveryCompleted { vault_id });
                }

                Ok(())
            })?;

            Self::deposit_event(Event::ShareRevealed { vault_id, share_id });

            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::lock_vault())]
        pub fn lock_vault(origin: OriginFor<T>, vault_id: VaultId) -> DispatchResult {
            ensure_signed(origin)?;

            Vaults::<T>::try_mutate(vault_id, |vault| -> DispatchResult {
                let v = vault.as_mut().ok_or(Error::<T>::VaultNotFound)?;

                ensure!(v.status == VaultStatus::Active, Error::<T>::VaultNotActive);

                v.status = VaultStatus::Locked;
                v.last_activity = frame_system::Pallet::<T>::block_number();

                Self::deposit_event(Event::VaultLocked { vault_id });

                Ok(())
            })
        }

        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::dissolve_vault())]
        pub fn dissolve_vault(origin: OriginFor<T>, vault_id: VaultId) -> DispatchResult {
            ensure_root(origin)?;

            Vaults::<T>::try_mutate(vault_id, |vault| -> DispatchResult {
                let v = vault.as_mut().ok_or(Error::<T>::VaultNotFound)?;

                ensure!(
                    v.status != VaultStatus::Active,
                    Error::<T>::CannotDissolvActiveVault
                );

                if v.status == VaultStatus::Active {
                    ActiveVaultCount::<T>::mutate(|count| *count = count.saturating_sub(1));
                }

                v.status = VaultStatus::Dissolved;
                v.last_activity = frame_system::Pallet::<T>::block_number();

                Self::deposit_event(Event::VaultDissolved { vault_id });

                Ok(())
            })
        }
    }

    impl<T: Config> Pallet<T> {
        fn next_vault_id() -> VaultId {
            let id = VaultCount::<T>::get();
            VaultCount::<T>::put(id.saturating_add(1));
            VaultId::new(id)
        }

        fn next_share_id() -> ShareId {
            let id = ShareCount::<T>::get();
            ShareCount::<T>::put(id.saturating_add(1));
            ShareId::new(id)
        }

        fn account_to_actor(account: T::AccountId) -> ActorId {
            let encoded = account.encode();
            let mut bytes = [0u8; 32];
            let len = encoded.len().min(32);
            bytes[..len].copy_from_slice(&encoded[..len]);
            ActorId::from_raw(bytes)
        }

        pub fn get_vault_members(vault_id: VaultId) -> Vec<ActorId> {
            VaultMembers::<T>::iter_prefix(vault_id)
                .map(|(actor, _)| actor)
                .collect()
        }

        pub fn get_vault_shares(vault_id: VaultId) -> Vec<ShareId> {
            VaultShares::<T>::iter_prefix(vault_id)
                .map(|(share_id, _)| share_id)
                .collect()
        }

        pub fn is_vault_active(vault_id: VaultId) -> bool {
            Vaults::<T>::get(vault_id)
                .is_some_and(|v| v.status == VaultStatus::Active)
        }

        pub fn get_revealed_shares_count(vault_id: VaultId) -> u32 {
            RecoveryRequests::<T>::get(vault_id)
                .map(|r| r.shares_revealed)
                .unwrap_or(0)
        }

        pub fn get_total_active_vaults() -> u32 {
            ActiveVaultCount::<T>::get()
        }

        pub fn is_recovery_active(vault_id: VaultId) -> bool {
            Vaults::<T>::get(vault_id)
                .is_some_and(|v| v.status == VaultStatus::Recovering)
        }
    }
}
