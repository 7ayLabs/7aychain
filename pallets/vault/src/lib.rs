#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::expect_used)]
extern crate alloc;

pub use pallet::*;
pub mod weights;

#[cfg(test)]
mod tests;

use alloc::vec::Vec;
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
    Default,
    Hash,
)]
pub struct VaultId(pub u64);

impl VaultId {
    pub fn new(id: u64) -> Self {
        Self(id)
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
    Default,
    Hash,
)]
pub struct ShareId(pub u64);

impl ShareId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum VaultStatus {
    #[default]
    Creating,
    Active,
    Locked,
    Recovering,
    Dissolved,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum MemberRole {
    Owner,
    Guardian,
    #[default]
    Participant,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum ShareStatus {
    #[default]
    Pending,
    Distributed,
    Revealed,
    Invalidated,
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
pub struct VaultMember<T: Config> {
    pub vault: VaultId,
    pub actor: ActorId,
    pub role: MemberRole,
    pub share_index: u32,
    pub joined_at: BlockNumberFor<T>,
    pub share_committed: bool,
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
pub struct Share<T: Config> {
    pub id: ShareId,
    pub vault: VaultId,
    pub holder: ActorId,
    pub index: u32,
    pub commitment: H256,
    pub status: ShareStatus,
    pub created_at: BlockNumberFor<T>,
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
pub struct RecoveryRequest<T: Config> {
    pub vault: VaultId,
    pub requester: ActorId,
    pub shares_revealed: u32,
    pub initiated_at: BlockNumberFor<T>,
    pub expires_at: BlockNumberFor<T>,
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
    Default,
    Hash,
)]
pub struct UnlockRequestId(pub u64);

impl UnlockRequestId {
    pub fn new(id: u64) -> Self {
        Self(id)
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
pub struct VaultFile<T: Config> {
    pub vault: VaultId,
    pub enc_hash: H256,
    pub plaintext_hash: H256,
    pub key_fingerprint: H256,
    pub size_bytes: u64,
    pub registered_by: ActorId,
    pub registered_at: BlockNumberFor<T>,
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
pub struct UnlockRequest<T: Config> {
    pub id: UnlockRequestId,
    pub vault: VaultId,
    pub file_enc_hash: H256,
    pub requester: ActorId,
    pub approvals: u32,
    pub initiated_at: BlockNumberFor<T>,
    pub expires_at: BlockNumberFor<T>,
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
        type MinThreshold: Get<u32>;

        #[pallet::constant]
        type MinRingSize: Get<u32>;

        #[pallet::constant]
        type MaxRingSize: Get<u32>;

        #[pallet::constant]
        type RecoveryPeriodBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MaxVaultsPerActor: Get<u32>;

        #[pallet::constant]
        type MaxFilesPerVault: Get<u32>;

        #[pallet::constant]
        type UnlockPeriodBlocks: Get<BlockNumberFor<Self>>;
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

    #[pallet::storage]
    #[pallet::getter(fn vault_files)]
    pub type VaultFiles<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, VaultId, Blake2_128Concat, H256, VaultFile<T>>;

    #[pallet::storage]
    #[pallet::getter(fn vault_file_count)]
    pub type VaultFileCount<T: Config> = StorageMap<_, Blake2_128Concat, VaultId, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn unlock_request_count)]
    pub type UnlockRequestCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn unlock_requests)]
    pub type UnlockRequests<T: Config> =
        StorageMap<_, Blake2_128Concat, UnlockRequestId, UnlockRequest<T>>;

    #[pallet::storage]
    #[pallet::getter(fn unlock_approvals)]
    pub type UnlockApprovals<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, UnlockRequestId, Blake2_128Concat, ActorId, ()>;

    #[pallet::storage]
    #[pallet::getter(fn active_unlocks)]
    pub type ActiveUnlocks<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, VaultId, Blake2_128Concat, H256, UnlockRequestId>;

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
            UnlockRequestCount::<T>::put(0u64);
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
        FileRegistered {
            vault_id: VaultId,
            enc_hash: H256,
            key_fingerprint: H256,
            registered_by: ActorId,
        },
        UnlockRequested {
            vault_id: VaultId,
            request_id: UnlockRequestId,
            file_enc_hash: H256,
            requester: ActorId,
        },
        UnlockAuthorized {
            vault_id: VaultId,
            request_id: UnlockRequestId,
            actor: ActorId,
            approvals_so_far: u32,
        },
        FileUnlockCompleted {
            vault_id: VaultId,
            request_id: UnlockRequestId,
            file_enc_hash: H256,
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
        NotShareHolder,
        FileAlreadyRegistered,
        FileNotFound,
        UnlockAlreadyActive,
        AlreadyApproved,
        UnlockExpired,
        MaxFilesReached,
        UnlockNotFound,
        UnlockAlreadyCompleted,
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
            let who = ensure_signed(origin)?;
            let caller_actor = Self::account_to_actor(who);
            ensure!(caller_actor == owner, Error::<T>::NotVaultOwner);

            ensure!(
                threshold >= T::MinThreshold::get(),
                Error::<T>::InvalidThreshold
            );
            ensure!(
                ring_size >= T::MinRingSize::get() && ring_size <= T::MaxRingSize::get(),
                Error::<T>::InvalidRingSize
            );
            ensure!(threshold <= ring_size, Error::<T>::ThresholdExceedsRingSize);

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
            let who = ensure_signed(origin)?;
            let caller_actor = Self::account_to_actor(who);

            let mut vault = Vaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;

            ensure!(vault.owner == caller_actor, Error::<T>::NotVaultOwner);
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

            // H16: check per-actor vault limit for new member
            let member_vault_count = VaultCountPerActor::<T>::get(member);
            ensure!(
                member_vault_count < T::MaxVaultsPerActor::get(),
                Error::<T>::MaxVaultsReached
            );

            Vaults::<T>::insert(vault_id, vault);
            VaultMembers::<T>::insert(vault_id, member, vault_member);
            ActorVaults::<T>::insert(member, vault_id, ());
            // H16: increment VaultCountPerActor for new member
            VaultCountPerActor::<T>::mutate(member, |c| *c = c.saturating_add(1));

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
            let who = ensure_signed(origin)?;
            let caller_actor = Self::account_to_actor(who);

            Vaults::<T>::try_mutate(vault_id, |vault| -> DispatchResult {
                let v = vault.as_mut().ok_or(Error::<T>::VaultNotFound)?;

                ensure!(v.owner == caller_actor, Error::<T>::NotVaultOwner);
                ensure!(
                    v.status == VaultStatus::Creating,
                    Error::<T>::VaultAlreadyActive
                );
                ensure!(v.member_count >= v.ring_size, Error::<T>::InvalidRingSize);

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

            let mut vault = Vaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;
            let actor = Self::account_to_actor(who);
            let block_number = frame_system::Pallet::<T>::block_number();

            if let Some(existing) = RecoveryRequests::<T>::get(vault_id) {
                if block_number <= existing.expires_at {
                    return Err(Error::<T>::RecoveryAlreadyActive.into());
                }
                RecoveryRequests::<T>::remove(vault_id);
                if vault.status == VaultStatus::Recovering {
                    vault.status = VaultStatus::Active;
                }
            }
            ensure!(
                vault.status == VaultStatus::Active,
                Error::<T>::VaultNotActive
            );
            ensure!(
                VaultMembers::<T>::contains_key(vault_id, actor),
                Error::<T>::NotVaultMember
            );

            let expires_at = block_number.saturating_add(T::RecoveryPeriodBlocks::get());

            let request = RecoveryRequest {
                vault: vault_id,
                requester: actor,
                shares_revealed: 0,
                initiated_at: block_number,
                expires_at,
            };

            RecoveryRequests::<T>::insert(vault_id, request);

            vault.status = VaultStatus::Recovering;
            vault.last_activity = block_number;
            Vaults::<T>::insert(vault_id, vault);

            Self::deposit_event(Event::RecoveryInitiated {
                vault_id,
                requester: actor,
            });

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::reveal_share())]
        pub fn reveal_share(origin: OriginFor<T>, share_id: ShareId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let caller_actor = Self::account_to_actor(who);

            let share = Shares::<T>::get(share_id).ok_or(Error::<T>::ShareNotFound)?;
            ensure!(share.holder == caller_actor, Error::<T>::NotShareHolder);
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
            let recovery_complete = RecoveryRequests::<T>::try_mutate(
                vault_id,
                |request| -> Result<bool, DispatchError> {
                    let r = request.as_mut().ok_or(Error::<T>::RecoveryNotActive)?;

                    ensure!(block_number <= r.expires_at, Error::<T>::RecoveryExpired);

                    r.shares_revealed = r.shares_revealed.saturating_add(1);

                    Ok(r.shares_revealed >= vault.threshold)
                },
            )?;

            Shares::<T>::mutate(share_id, |s| {
                if let Some(ref mut sh) = s {
                    sh.status = ShareStatus::Revealed;
                }
            });

            if recovery_complete {
                RecoveryRequests::<T>::remove(vault_id);
                Vaults::<T>::mutate(vault_id, |v| {
                    if let Some(ref mut vault) = v {
                        vault.status = VaultStatus::Active;
                        vault.last_activity = block_number;
                    }
                });

                Self::deposit_event(Event::RecoveryCompleted { vault_id });
            }

            Self::deposit_event(Event::ShareRevealed { vault_id, share_id });

            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::lock_vault())]
        pub fn lock_vault(origin: OriginFor<T>, vault_id: VaultId) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let caller_actor = Self::account_to_actor(who);

            Vaults::<T>::try_mutate(vault_id, |vault| -> DispatchResult {
                let v = vault.as_mut().ok_or(Error::<T>::VaultNotFound)?;

                ensure!(v.owner == caller_actor, Error::<T>::NotVaultOwner);
                ensure!(v.status == VaultStatus::Active, Error::<T>::VaultNotActive);

                v.status = VaultStatus::Locked;
                v.last_activity = frame_system::Pallet::<T>::block_number();

                ActiveVaultCount::<T>::mutate(|count| {
                    *count = count.saturating_sub(1);
                });

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

                // M19: clean up members and decrement their VaultCountPerActor
                for (actor, _) in VaultMembers::<T>::drain_prefix(vault_id) {
                    ActorVaults::<T>::remove(actor, vault_id);
                    VaultCountPerActor::<T>::mutate(actor, |c| *c = c.saturating_sub(1));
                }

                // M19: clean up shares
                for (share_id, _) in VaultShares::<T>::drain_prefix(vault_id) {
                    if let Some(share) = Shares::<T>::take(share_id) {
                        ActorShares::<T>::remove(share.holder, share_id);
                    }
                }

                // M19: clean up files
                let _ = VaultFiles::<T>::clear_prefix(vault_id, u32::MAX, None);
                VaultFileCount::<T>::remove(vault_id);

                // M19: clean up recovery requests
                RecoveryRequests::<T>::remove(vault_id);

                Self::deposit_event(Event::VaultDissolved { vault_id });

                Ok(())
            })
        }

        /// Register an encrypted file reference in a vault.
        ///
        /// Guards: vault must be Active, caller must be a member,
        /// file must not already exist, and file count must be
        /// under `MaxFilesPerVault`.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::register_file())]
        pub fn register_file(
            origin: OriginFor<T>,
            vault_id: VaultId,
            enc_hash: H256,
            plaintext_hash: H256,
            key_fingerprint: H256,
            size_bytes: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let vault = Vaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;
            ensure!(
                vault.status == VaultStatus::Active,
                Error::<T>::VaultNotActive
            );

            let actor = Self::account_to_actor(who);
            ensure!(
                VaultMembers::<T>::contains_key(vault_id, actor),
                Error::<T>::NotVaultMember
            );

            ensure!(
                !VaultFiles::<T>::contains_key(vault_id, enc_hash),
                Error::<T>::FileAlreadyRegistered
            );

            let file_count = VaultFileCount::<T>::get(vault_id);
            ensure!(
                file_count < T::MaxFilesPerVault::get(),
                Error::<T>::MaxFilesReached
            );

            let block_number = frame_system::Pallet::<T>::block_number();

            let file = VaultFile {
                vault: vault_id,
                enc_hash,
                plaintext_hash,
                key_fingerprint,
                size_bytes,
                registered_by: actor,
                registered_at: block_number,
            };

            VaultFiles::<T>::insert(vault_id, enc_hash, file);
            VaultFileCount::<T>::insert(vault_id, file_count.saturating_add(1));

            Vaults::<T>::mutate(vault_id, |v| {
                if let Some(ref mut vault) = v {
                    vault.last_activity = block_number;
                }
            });

            Self::deposit_event(Event::FileRegistered {
                vault_id,
                enc_hash,
                key_fingerprint,
                registered_by: actor,
            });

            Ok(())
        }

        /// Request a threshold-gated unlock of a registered file.
        ///
        /// Guards: vault Active, caller is member, file exists,
        /// no active unlock already in progress for this file.
        /// The requester auto-approves (approvals starts at 1).
        /// If threshold is 1, the unlock completes immediately.
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::request_unlock())]
        pub fn request_unlock(
            origin: OriginFor<T>,
            vault_id: VaultId,
            file_enc_hash: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let vault = Vaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;
            ensure!(
                vault.status == VaultStatus::Active,
                Error::<T>::VaultNotActive
            );

            let actor = Self::account_to_actor(who);
            ensure!(
                VaultMembers::<T>::contains_key(vault_id, actor),
                Error::<T>::NotVaultMember
            );

            ensure!(
                VaultFiles::<T>::contains_key(vault_id, file_enc_hash),
                Error::<T>::FileNotFound
            );

            ensure!(
                !ActiveUnlocks::<T>::contains_key(vault_id, file_enc_hash),
                Error::<T>::UnlockAlreadyActive
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            let expires_at = block_number.saturating_add(T::UnlockPeriodBlocks::get());
            let request_id = Self::next_unlock_request_id();

            let request = UnlockRequest {
                id: request_id,
                vault: vault_id,
                file_enc_hash,
                requester: actor,
                approvals: 1,
                initiated_at: block_number,
                expires_at,
                completed: false,
            };

            UnlockRequests::<T>::insert(request_id, request);
            UnlockApprovals::<T>::insert(request_id, actor, ());
            ActiveUnlocks::<T>::insert(vault_id, file_enc_hash, request_id);

            Self::deposit_event(Event::UnlockRequested {
                vault_id,
                request_id,
                file_enc_hash,
                requester: actor,
            });

            // Auto-complete if threshold is already met
            if 1u32 >= vault.threshold {
                Self::complete_unlock(vault_id, request_id, file_enc_hash)?;
            }

            Ok(())
        }

        /// Authorize (approve) an existing unlock request.
        ///
        /// Guards: request exists, not completed, not expired,
        /// caller is vault member, caller has not already
        /// approved. If approvals reach threshold, auto-complete.
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::authorize_unlock())]
        pub fn authorize_unlock(
            origin: OriginFor<T>,
            request_id: UnlockRequestId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let request = UnlockRequests::<T>::get(request_id).ok_or(Error::<T>::UnlockNotFound)?;

            ensure!(!request.completed, Error::<T>::UnlockAlreadyCompleted);

            let block_number = frame_system::Pallet::<T>::block_number();
            ensure!(
                block_number <= request.expires_at,
                Error::<T>::UnlockExpired
            );

            let vault_id = request.vault;
            let vault = Vaults::<T>::get(vault_id).ok_or(Error::<T>::VaultNotFound)?;

            let actor = Self::account_to_actor(who);
            ensure!(
                VaultMembers::<T>::contains_key(vault_id, actor),
                Error::<T>::NotVaultMember
            );

            ensure!(
                !UnlockApprovals::<T>::contains_key(request_id, actor),
                Error::<T>::AlreadyApproved
            );

            UnlockApprovals::<T>::insert(request_id, actor, ());

            let new_approvals =
                UnlockRequests::<T>::try_mutate(request_id, |req| -> Result<u32, DispatchError> {
                    let r = req.as_mut().ok_or(Error::<T>::UnlockNotFound)?;
                    r.approvals = r.approvals.saturating_add(1);
                    Ok(r.approvals)
                })?;

            Self::deposit_event(Event::UnlockAuthorized {
                vault_id,
                request_id,
                actor,
                approvals_so_far: new_approvals,
            });

            if new_approvals >= vault.threshold {
                Self::complete_unlock(vault_id, request_id, request.file_enc_hash)?;
            }

            Ok(())
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

        fn next_unlock_request_id() -> UnlockRequestId {
            let id = UnlockRequestCount::<T>::get();
            UnlockRequestCount::<T>::put(id.saturating_add(1));
            UnlockRequestId::new(id)
        }

        fn complete_unlock(
            vault_id: VaultId,
            request_id: UnlockRequestId,
            file_enc_hash: H256,
        ) -> DispatchResult {
            UnlockRequests::<T>::mutate(request_id, |req| {
                if let Some(ref mut r) = req {
                    r.completed = true;
                }
            });
            ActiveUnlocks::<T>::remove(vault_id, file_enc_hash);
            Self::deposit_event(Event::FileUnlockCompleted {
                vault_id,
                request_id,
                file_enc_hash,
            });
            Ok(())
        }

        fn account_to_actor(account: T::AccountId) -> ActorId {
            seveny_primitives::crypto::derive_actor_id(&account.encode())
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
            Vaults::<T>::get(vault_id).is_some_and(|v| v.status == VaultStatus::Active)
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
            Vaults::<T>::get(vault_id).is_some_and(|v| v.status == VaultStatus::Recovering)
        }
    }
}
