#![allow(clippy::disallowed_macros)]

use crate::{self as pallet_vault, Error, Event, MemberRole, ShareStatus, VaultId, VaultStatus};
use frame_support::{assert_noop, assert_ok, derive_impl, parameter_types, traits::ConstU32};
use frame_system as system;
use seveny_primitives::types::ActorId;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Vault: pallet_vault,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const MinThreshold: u32 = 2;
    pub const MinRingSize: u32 = 3;
    pub const MaxRingSize: u32 = 10;
    pub const RecoveryPeriodBlocks: u64 = 100;
    pub const MaxVaultsPerActor: u32 = 5;
}

impl pallet_vault::Config for Test {
    type WeightInfo = ();
    type MinThreshold = MinThreshold;
    type MinRingSize = MinRingSize;
    type MaxRingSize = MaxRingSize;
    type RecoveryPeriodBlocks = RecoveryPeriodBlocks;
    type MaxVaultsPerActor = MaxVaultsPerActor;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_vault::GenesisConfig::<Test> {
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("genesis build failed");

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn account_to_actor(account: u64) -> ActorId {
    use parity_scale_codec::Encode;
    let encoded = account.encode();
    let hash = sp_core::blake2_256(&encoded);
    ActorId::from_raw(hash)
}

fn create_vault_with_members(owner: u64, member_count: u32) -> VaultId {
    let owner_actor = account_to_actor(owner);

    Vault::create_vault(
        RuntimeOrigin::signed(owner),
        owner_actor,
        2,
        member_count,
        H256([1u8; 32]),
    )
    .expect("vault creation failed");

    let vault_id = VaultId::new(0);

    for i in 1..member_count {
        Vault::add_member(
            RuntimeOrigin::signed(owner),
            vault_id,
            account_to_actor(owner + i as u64),
            MemberRole::Participant,
        )
        .expect("add member failed");
    }

    vault_id
}

#[test]
fn create_vault_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Vault::create_vault(
            RuntimeOrigin::signed(1),
            owner,
            2,
            3,
            H256([1u8; 32])
        ));

        let vault_id = VaultId::new(0);
        let vault = Vault::vaults(vault_id).expect("vault should exist");

        assert_eq!(vault.owner, owner);
        assert_eq!(vault.threshold, 2);
        assert_eq!(vault.ring_size, 3);
        assert_eq!(vault.status, VaultStatus::Creating);
        assert_eq!(vault.member_count, 1);
    });
}

#[test]
fn invalid_threshold_rejected() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_noop!(
            Vault::create_vault(RuntimeOrigin::signed(1), owner, 1, 3, H256([1u8; 32])),
            Error::<Test>::InvalidThreshold
        );
    });
}

#[test]
fn invalid_ring_size_rejected() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_noop!(
            Vault::create_vault(RuntimeOrigin::signed(1), owner, 2, 2, H256([1u8; 32])),
            Error::<Test>::InvalidRingSize
        );

        assert_noop!(
            Vault::create_vault(RuntimeOrigin::signed(1), owner, 2, 11, H256([1u8; 32])),
            Error::<Test>::InvalidRingSize
        );
    });
}

#[test]
fn threshold_exceeds_ring_size_rejected() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_noop!(
            Vault::create_vault(RuntimeOrigin::signed(1), owner, 5, 3, H256([1u8; 32])),
            Error::<Test>::ThresholdExceedsRingSize
        );
    });
}

#[test]
fn add_member_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let member = account_to_actor(2);

        assert_ok!(Vault::create_vault(
            RuntimeOrigin::signed(1),
            owner,
            2,
            3,
            H256([1u8; 32])
        ));

        let vault_id = VaultId::new(0);

        assert_ok!(Vault::add_member(
            RuntimeOrigin::signed(1),
            vault_id,
            member,
            MemberRole::Guardian
        ));

        let vault = Vault::vaults(vault_id).expect("vault should exist");
        assert_eq!(vault.member_count, 2);

        let member_info = Vault::vault_members(vault_id, member).expect("member should exist");
        assert_eq!(member_info.role, MemberRole::Guardian);
        assert_eq!(member_info.share_index, 1);
    });
}

#[test]
fn cannot_add_duplicate_member() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let member = account_to_actor(2);

        assert_ok!(Vault::create_vault(
            RuntimeOrigin::signed(1),
            owner,
            2,
            3,
            H256([1u8; 32])
        ));

        let vault_id = VaultId::new(0);

        assert_ok!(Vault::add_member(
            RuntimeOrigin::signed(1),
            vault_id,
            member,
            MemberRole::Participant
        ));

        assert_noop!(
            Vault::add_member(
                RuntimeOrigin::signed(1),
                vault_id,
                member,
                MemberRole::Guardian
            ),
            Error::<Test>::MemberAlreadyExists
        );
    });
}

#[test]
fn activate_vault_success() {
    new_test_ext().execute_with(|| {
        let vault_id = create_vault_with_members(1, 3);

        assert_ok!(Vault::activate_vault(RuntimeOrigin::signed(1), vault_id));

        let vault = Vault::vaults(vault_id).expect("vault should exist");
        assert_eq!(vault.status, VaultStatus::Active);
        assert_eq!(Vault::get_total_active_vaults(), 1);
    });
}

#[test]
fn cannot_activate_incomplete_vault() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Vault::create_vault(
            RuntimeOrigin::signed(1),
            owner,
            2,
            3,
            H256([1u8; 32])
        ));

        let vault_id = VaultId::new(0);

        assert_noop!(
            Vault::activate_vault(RuntimeOrigin::signed(1), vault_id),
            Error::<Test>::InvalidRingSize
        );
    });
}

#[test]
fn commit_share_success() {
    new_test_ext().execute_with(|| {
        let vault_id = create_vault_with_members(1, 3);
        assert_ok!(Vault::activate_vault(RuntimeOrigin::signed(1), vault_id));

        assert_ok!(Vault::commit_share(
            RuntimeOrigin::signed(1),
            vault_id,
            H256([2u8; 32])
        ));

        let shares = Vault::get_vault_shares(vault_id);
        assert_eq!(shares.len(), 1);

        let share = Vault::shares(shares[0]).expect("share should exist");
        assert_eq!(share.status, ShareStatus::Distributed);
    });
}

#[test]
fn cannot_commit_share_twice() {
    new_test_ext().execute_with(|| {
        let vault_id = create_vault_with_members(1, 3);
        assert_ok!(Vault::activate_vault(RuntimeOrigin::signed(1), vault_id));

        assert_ok!(Vault::commit_share(
            RuntimeOrigin::signed(1),
            vault_id,
            H256([2u8; 32])
        ));

        assert_noop!(
            Vault::commit_share(RuntimeOrigin::signed(1), vault_id, H256([3u8; 32])),
            Error::<Test>::ShareAlreadyCommitted
        );
    });
}

#[test]
fn initiate_recovery_success() {
    new_test_ext().execute_with(|| {
        let vault_id = create_vault_with_members(1, 3);
        assert_ok!(Vault::activate_vault(RuntimeOrigin::signed(1), vault_id));

        assert_ok!(Vault::initiate_recovery(RuntimeOrigin::signed(1), vault_id));

        let vault = Vault::vaults(vault_id).expect("vault should exist");
        assert_eq!(vault.status, VaultStatus::Recovering);
        assert!(Vault::is_recovery_active(vault_id));
    });
}

#[test]
fn cannot_initiate_recovery_twice() {
    new_test_ext().execute_with(|| {
        let vault_id = create_vault_with_members(1, 3);
        assert_ok!(Vault::activate_vault(RuntimeOrigin::signed(1), vault_id));

        assert_ok!(Vault::initiate_recovery(RuntimeOrigin::signed(1), vault_id));

        assert_noop!(
            Vault::initiate_recovery(RuntimeOrigin::signed(1), vault_id),
            Error::<Test>::RecoveryAlreadyActive
        );
    });
}

#[test]
fn reveal_share_success() {
    new_test_ext().execute_with(|| {
        let vault_id = create_vault_with_members(1, 3);
        assert_ok!(Vault::activate_vault(RuntimeOrigin::signed(1), vault_id));

        assert_ok!(Vault::commit_share(
            RuntimeOrigin::signed(1),
            vault_id,
            H256([2u8; 32])
        ));

        assert_ok!(Vault::initiate_recovery(RuntimeOrigin::signed(1), vault_id));

        let shares = Vault::get_vault_shares(vault_id);
        assert_ok!(Vault::reveal_share(RuntimeOrigin::signed(1), shares[0]));

        let share = Vault::shares(shares[0]).expect("share should exist");
        assert_eq!(share.status, ShareStatus::Revealed);
        assert_eq!(Vault::get_revealed_shares_count(vault_id), 1);
    });
}

#[test]
fn recovery_completes_at_threshold() {
    new_test_ext().execute_with(|| {
        let vault_id = create_vault_with_members(1, 3);
        assert_ok!(Vault::activate_vault(RuntimeOrigin::signed(1), vault_id));

        assert_ok!(Vault::commit_share(
            RuntimeOrigin::signed(1),
            vault_id,
            H256([2u8; 32])
        ));
        assert_ok!(Vault::commit_share(
            RuntimeOrigin::signed(2),
            vault_id,
            H256([3u8; 32])
        ));

        assert_ok!(Vault::initiate_recovery(RuntimeOrigin::signed(1), vault_id));

        let shares = Vault::get_vault_shares(vault_id);

        assert_ok!(Vault::reveal_share(RuntimeOrigin::signed(1), shares[0]));
        assert_ok!(Vault::reveal_share(RuntimeOrigin::signed(2), shares[1]));

        let vault = Vault::vaults(vault_id).expect("vault should exist");
        assert_eq!(vault.status, VaultStatus::Active);
    });
}

#[test]
fn lock_vault_success() {
    new_test_ext().execute_with(|| {
        let vault_id = create_vault_with_members(1, 3);
        assert_ok!(Vault::activate_vault(RuntimeOrigin::signed(1), vault_id));

        assert_ok!(Vault::lock_vault(RuntimeOrigin::signed(1), vault_id));

        let vault = Vault::vaults(vault_id).expect("vault should exist");
        assert_eq!(vault.status, VaultStatus::Locked);
    });
}

#[test]
fn dissolve_vault_success() {
    new_test_ext().execute_with(|| {
        let vault_id = create_vault_with_members(1, 3);
        assert_ok!(Vault::activate_vault(RuntimeOrigin::signed(1), vault_id));
        assert_ok!(Vault::lock_vault(RuntimeOrigin::signed(1), vault_id));

        assert_ok!(Vault::dissolve_vault(RuntimeOrigin::root(), vault_id));

        let vault = Vault::vaults(vault_id).expect("vault should exist");
        assert_eq!(vault.status, VaultStatus::Dissolved);
    });
}

#[test]
fn cannot_dissolve_active_vault() {
    new_test_ext().execute_with(|| {
        let vault_id = create_vault_with_members(1, 3);
        assert_ok!(Vault::activate_vault(RuntimeOrigin::signed(1), vault_id));

        assert_noop!(
            Vault::dissolve_vault(RuntimeOrigin::root(), vault_id),
            Error::<Test>::CannotDissolvActiveVault
        );
    });
}

#[test]
fn max_vaults_per_actor_enforced() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        for i in 0..5 {
            assert_ok!(Vault::create_vault(
                RuntimeOrigin::signed(1),
                owner,
                2,
                3,
                H256([i as u8; 32])
            ));
        }

        assert_noop!(
            Vault::create_vault(RuntimeOrigin::signed(1), owner, 2, 3, H256([100u8; 32])),
            Error::<Test>::MaxVaultsReached
        );
    });
}

#[test]
fn get_vault_members_helper() {
    new_test_ext().execute_with(|| {
        let vault_id = create_vault_with_members(1, 3);

        let members = Vault::get_vault_members(vault_id);
        assert_eq!(members.len(), 3);
    });
}

#[test]
fn member_roles() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let guardian = account_to_actor(2);
        let participant = account_to_actor(3);

        assert_ok!(Vault::create_vault(
            RuntimeOrigin::signed(1),
            owner,
            2,
            3,
            H256([1u8; 32])
        ));

        let vault_id = VaultId::new(0);

        assert_ok!(Vault::add_member(
            RuntimeOrigin::signed(1),
            vault_id,
            guardian,
            MemberRole::Guardian
        ));

        assert_ok!(Vault::add_member(
            RuntimeOrigin::signed(1),
            vault_id,
            participant,
            MemberRole::Participant
        ));

        let owner_member = Vault::vault_members(vault_id, owner).expect("owner should exist");
        assert_eq!(owner_member.role, MemberRole::Owner);

        let guardian_member =
            Vault::vault_members(vault_id, guardian).expect("guardian should exist");
        assert_eq!(guardian_member.role, MemberRole::Guardian);

        let participant_member =
            Vault::vault_members(vault_id, participant).expect("participant should exist");
        assert_eq!(participant_member.role, MemberRole::Participant);
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Vault::create_vault(
            RuntimeOrigin::signed(1),
            owner,
            2,
            3,
            H256([1u8; 32])
        ));

        System::assert_has_event(RuntimeEvent::Vault(Event::VaultCreated {
            vault_id: VaultId::new(0),
            owner,
            threshold: 2,
            ring_size: 3,
        }));

        System::assert_has_event(RuntimeEvent::Vault(Event::MemberAdded {
            vault_id: VaultId::new(0),
            member: owner,
            role: MemberRole::Owner,
        }));
    });
}

#[test]
fn genesis_initializes_counts() {
    new_test_ext().execute_with(|| {
        assert_eq!(Vault::vault_count(), 0);
        assert_eq!(Vault::share_count(), 0);
        assert_eq!(Vault::get_total_active_vaults(), 0);
    });
}

#[test]
fn different_threshold_configurations() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Vault::create_vault(
            RuntimeOrigin::signed(1),
            owner,
            2,
            3,
            H256([1u8; 32])
        ));

        let vault = Vault::vaults(VaultId::new(0)).expect("vault should exist");
        assert_eq!(vault.threshold, 2);
        assert_eq!(vault.ring_size, 3);

        assert_ok!(Vault::create_vault(
            RuntimeOrigin::signed(1),
            owner,
            3,
            5,
            H256([2u8; 32])
        ));

        let vault2 = Vault::vaults(VaultId::new(1)).expect("vault should exist");
        assert_eq!(vault2.threshold, 3);
        assert_eq!(vault2.ring_size, 5);

        assert_ok!(Vault::create_vault(
            RuntimeOrigin::signed(1),
            owner,
            5,
            10,
            H256([3u8; 32])
        ));

        let vault3 = Vault::vaults(VaultId::new(2)).expect("vault should exist");
        assert_eq!(vault3.threshold, 5);
        assert_eq!(vault3.ring_size, 10);
    });
}
