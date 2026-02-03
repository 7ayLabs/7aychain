#![allow(clippy::disallowed_macros)]

use crate::{
    self as pallet_boomerang, Error, Event, HopDirection, PathFailureReason, PathId, PathStatus,
};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, Hooks},
};
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
        Boomerang: pallet_boomerang,
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
    pub const BoomerangTimeoutBlocks: u64 = 30;
    pub const MaxExtensionBlocks: u64 = 60;
    pub const MaxHopsPerPath: u32 = 10;
    pub const MaxActivePaths: u32 = 100;
}

impl pallet_boomerang::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type BoomerangTimeoutBlocks = BoomerangTimeoutBlocks;
    type MaxExtensionBlocks = MaxExtensionBlocks;
    type MaxHopsPerPath = MaxHopsPerPath;
    type MaxActivePaths = MaxActivePaths;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_boomerang::GenesisConfig::<Test> {
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("genesis build failed");

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn account_to_actor(account: u64) -> ActorId {
    let mut bytes = [0u8; 32];
    bytes[0..8].copy_from_slice(&account.to_le_bytes());
    ActorId::from_raw(bytes)
}

#[test]
fn initiate_path_success() {
    new_test_ext().execute_with(|| {
        let target = account_to_actor(2);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);
        let path = Boomerang::paths(path_id).expect("path should exist");

        assert_eq!(path.target, target);
        assert_eq!(path.status, PathStatus::Initiated);
        assert_eq!(path.outbound_hops, 0);
        assert_eq!(path.return_hops, 0);
        assert_eq!(Boomerang::get_active_path_count(), 1);
    });
}

#[test]
fn self_path_prevented() {
    new_test_ext().execute_with(|| {
        let self_actor = account_to_actor(1);

        assert_noop!(
            Boomerang::initiate_path(RuntimeOrigin::signed(1), self_actor),
            Error::<Test>::SelfPath
        );
    });
}

#[test]
fn record_outbound_hop_success() {
    new_test_ext().execute_with(|| {
        let target = account_to_actor(2);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);

        assert_ok!(Boomerang::record_hop(
            RuntimeOrigin::signed(1),
            path_id,
            target,
            H256([1u8; 32])
        ));

        let path = Boomerang::paths(path_id).expect("path should exist");
        assert_eq!(path.outbound_hops, 1);
        assert_eq!(path.status, PathStatus::AwaitingReturn);
    });
}

#[test]
fn record_hop_in_progress() {
    new_test_ext().execute_with(|| {
        let intermediate = account_to_actor(2);
        let target = account_to_actor(3);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);

        assert_ok!(Boomerang::record_hop(
            RuntimeOrigin::signed(1),
            path_id,
            intermediate,
            H256([1u8; 32])
        ));

        let path = Boomerang::paths(path_id).expect("path should exist");
        assert_eq!(path.status, PathStatus::InProgress);
        assert_eq!(path.outbound_hops, 1);
    });
}

#[test]
fn full_boomerang_path_completion() {
    new_test_ext().execute_with(|| {
        let initiator = account_to_actor(1);
        let target = account_to_actor(2);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);

        assert_ok!(Boomerang::record_hop(
            RuntimeOrigin::signed(1),
            path_id,
            target,
            H256([1u8; 32])
        ));

        let path = Boomerang::paths(path_id).expect("path should exist");
        assert_eq!(path.status, PathStatus::AwaitingReturn);

        assert_ok!(Boomerang::record_hop(
            RuntimeOrigin::signed(2),
            path_id,
            initiator,
            H256([2u8; 32])
        ));

        let path = Boomerang::paths(path_id).expect("path should exist");
        assert_eq!(path.status, PathStatus::Completed);
        assert_eq!(path.outbound_hops, 1);
        assert_eq!(path.return_hops, 1);
        assert!(path.verification_hash.is_some());
        assert_eq!(Boomerang::get_active_path_count(), 0);
    });
}

#[test]
fn path_times_out() {
    new_test_ext().execute_with(|| {
        let target = account_to_actor(2);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);

        System::set_block_number(35);
        Boomerang::on_initialize(35);

        let path = Boomerang::paths(path_id).expect("path should exist");
        assert_eq!(path.status, PathStatus::TimedOut);
    });
}

#[test]
fn extend_timeout_success() {
    new_test_ext().execute_with(|| {
        let target = account_to_actor(2);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);

        assert_ok!(Boomerang::record_hop(
            RuntimeOrigin::signed(1),
            path_id,
            target,
            H256([1u8; 32])
        ));

        assert_ok!(Boomerang::extend_timeout(RuntimeOrigin::signed(1), path_id));

        let path = Boomerang::paths(path_id).expect("path should exist");
        assert!(path.extended_timeout_at.is_some());
        assert_eq!(path.extended_timeout_at, Some(91));
    });
}

#[test]
fn double_extension_prevented() {
    new_test_ext().execute_with(|| {
        let target = account_to_actor(2);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);

        assert_ok!(Boomerang::record_hop(
            RuntimeOrigin::signed(1),
            path_id,
            target,
            H256([1u8; 32])
        ));

        assert_ok!(Boomerang::extend_timeout(RuntimeOrigin::signed(1), path_id));

        assert_noop!(
            Boomerang::extend_timeout(RuntimeOrigin::signed(1), path_id),
            Error::<Test>::AlreadyExtended
        );
    });
}

#[test]
fn fail_path_success() {
    new_test_ext().execute_with(|| {
        let target = account_to_actor(2);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);

        assert_ok!(Boomerang::fail_path(
            RuntimeOrigin::root(),
            path_id,
            PathFailureReason::VerificationFailed
        ));

        let path = Boomerang::paths(path_id).expect("path should exist");
        assert_eq!(path.status, PathStatus::Failed);
        assert_eq!(Boomerang::get_active_path_count(), 0);
    });
}

#[test]
fn max_hops_enforced() {
    new_test_ext().execute_with(|| {
        let target = account_to_actor(11);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);

        for i in 2..12 {
            let to_actor = account_to_actor(i);
            assert_ok!(Boomerang::record_hop(
                RuntimeOrigin::signed((i - 1) as u64),
                path_id,
                to_actor,
                H256([(i as u8); 32])
            ));
        }

        assert_noop!(
            Boomerang::record_hop(
                RuntimeOrigin::signed(11),
                path_id,
                account_to_actor(1),
                H256([12u8; 32])
            ),
            Error::<Test>::MaxHopsReached
        );
    });
}

#[test]
fn is_path_active_helper() {
    new_test_ext().execute_with(|| {
        let target = account_to_actor(2);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);

        assert!(Boomerang::is_path_active(path_id));

        assert_ok!(Boomerang::fail_path(
            RuntimeOrigin::root(),
            path_id,
            PathFailureReason::InvalidHop
        ));

        assert!(!Boomerang::is_path_active(path_id));
    });
}

#[test]
fn verify_path_helper() {
    new_test_ext().execute_with(|| {
        let initiator = account_to_actor(1);
        let target = account_to_actor(2);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);

        assert!(!Boomerang::verify_path(path_id));

        assert_ok!(Boomerang::record_hop(
            RuntimeOrigin::signed(1),
            path_id,
            target,
            H256([1u8; 32])
        ));

        assert_ok!(Boomerang::record_hop(
            RuntimeOrigin::signed(2),
            path_id,
            initiator,
            H256([2u8; 32])
        ));

        assert!(Boomerang::verify_path(path_id));
    });
}

#[test]
fn get_path_hops_helper() {
    new_test_ext().execute_with(|| {
        let initiator = account_to_actor(1);
        let target = account_to_actor(2);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        let path_id = PathId::new(0);

        assert_ok!(Boomerang::record_hop(
            RuntimeOrigin::signed(1),
            path_id,
            target,
            H256([1u8; 32])
        ));

        assert_ok!(Boomerang::record_hop(
            RuntimeOrigin::signed(2),
            path_id,
            initiator,
            H256([2u8; 32])
        ));

        let hops = Boomerang::get_path_hops(path_id);
        assert_eq!(hops.len(), 2);
        assert_eq!(hops[0].direction, HopDirection::Outbound);
        assert_eq!(hops[1].direction, HopDirection::Return);
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        let initiator = account_to_actor(1);
        let target = account_to_actor(2);

        assert_ok!(Boomerang::initiate_path(RuntimeOrigin::signed(1), target));

        System::assert_has_event(RuntimeEvent::Boomerang(Event::PathInitiated {
            path_id: PathId::new(0),
            initiator,
            target,
            timeout_at: 31,
        }));
    });
}

#[test]
fn genesis_initializes_counts() {
    new_test_ext().execute_with(|| {
        assert_eq!(Boomerang::path_count(), 0);
        assert_eq!(Boomerang::get_active_path_count(), 0);
    });
}
