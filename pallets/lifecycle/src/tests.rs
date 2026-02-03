use crate::{self as pallet_lifecycle, *};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Lifecycle: pallet_lifecycle,
    }
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
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
    type BlockHashCount = ConstU64<250>;
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
    pub const KeyDestructionTimeoutBlocks: u64 = 50;
    pub const MinDestructionAttestations: u32 = 3;
    pub const RotationCooldownBlocks: u64 = 10;
}

impl pallet_lifecycle::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type KeyDestructionTimeoutBlocks = KeyDestructionTimeoutBlocks;
    type MinDestructionAttestations = MinDestructionAttestations;
    type RotationCooldownBlocks = RotationCooldownBlocks;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("failed to build storage");
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn create_key_hash(seed: u8) -> H256 {
    H256([seed; 32])
}

fn account_to_actor(account: u64) -> ActorId {
    let mut bytes = [0u8; 32];
    bytes[0] = account as u8;
    ActorId::from_raw(bytes)
}

fn register_and_activate(account: u64, key_hash: H256) {
    assert_ok!(Lifecycle::register_actor(
        RuntimeOrigin::signed(account),
        key_hash
    ));
    let actor = account_to_actor(account);
    assert_ok!(Lifecycle::activate_actor(RuntimeOrigin::root(), actor));
}

#[test]
fn register_actor_success() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);

        assert_ok!(Lifecycle::register_actor(
            RuntimeOrigin::signed(1),
            key_hash
        ));

        let actor = account_to_actor(1);
        let lifecycle = Lifecycle::actors(actor).expect("actor should exist");
        assert_eq!(lifecycle.status, ActorStatus::Pending);
        assert_eq!(lifecycle.key_hash, key_hash);
    });
}

#[test]
fn cannot_register_twice() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);

        assert_ok!(Lifecycle::register_actor(
            RuntimeOrigin::signed(1),
            key_hash
        ));

        assert_noop!(
            Lifecycle::register_actor(RuntimeOrigin::signed(1), key_hash),
            Error::<Test>::ActorAlreadyExists
        );
    });
}

#[test]
fn activate_actor_success() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);

        assert_ok!(Lifecycle::register_actor(
            RuntimeOrigin::signed(1),
            key_hash
        ));

        let actor = account_to_actor(1);
        assert_ok!(Lifecycle::activate_actor(RuntimeOrigin::root(), actor));

        let lifecycle = Lifecycle::actors(actor).expect("actor should exist");
        assert_eq!(lifecycle.status, ActorStatus::Active);
        assert!(Lifecycle::is_actor_active(actor));
    });
}

#[test]
fn suspend_actor_success() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);
        register_and_activate(1, key_hash);

        let actor = account_to_actor(1);
        assert_ok!(Lifecycle::suspend_actor(RuntimeOrigin::root(), actor));

        let lifecycle = Lifecycle::actors(actor).expect("actor should exist");
        assert_eq!(lifecycle.status, ActorStatus::Suspended);
        assert!(!Lifecycle::is_actor_active(actor));
    });
}

#[test]
fn reactivate_actor_success() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);
        register_and_activate(1, key_hash);

        let actor = account_to_actor(1);
        assert_ok!(Lifecycle::suspend_actor(RuntimeOrigin::root(), actor));
        assert_ok!(Lifecycle::reactivate_actor(RuntimeOrigin::root(), actor));

        let lifecycle = Lifecycle::actors(actor).expect("actor should exist");
        assert_eq!(lifecycle.status, ActorStatus::Active);
    });
}

#[test]
fn initiate_destruction_success() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);
        register_and_activate(1, key_hash);

        assert_ok!(Lifecycle::initiate_destruction(
            RuntimeOrigin::signed(1),
            DestructionReason::OwnerRequest
        ));

        let actor = account_to_actor(1);
        assert!(Lifecycle::is_destruction_pending(actor));

        let lifecycle = Lifecycle::actors(actor).expect("actor should exist");
        assert_eq!(lifecycle.status, ActorStatus::Destroying);
    });
}

#[test]
fn attest_destruction_success() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);
        register_and_activate(1, key_hash);
        register_and_activate(2, create_key_hash(2));

        assert_ok!(Lifecycle::initiate_destruction(
            RuntimeOrigin::signed(1),
            DestructionReason::OwnerRequest
        ));

        let actor = account_to_actor(1);
        assert_ok!(Lifecycle::attest_destruction(
            RuntimeOrigin::signed(2),
            actor,
            create_key_hash(99)
        ));

        assert_eq!(Lifecycle::get_attestation_count(actor), 1);
    });
}

#[test]
fn cannot_self_attest() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);
        register_and_activate(1, key_hash);

        assert_ok!(Lifecycle::initiate_destruction(
            RuntimeOrigin::signed(1),
            DestructionReason::OwnerRequest
        ));

        let actor = account_to_actor(1);
        assert_noop!(
            Lifecycle::attest_destruction(RuntimeOrigin::signed(1), actor, create_key_hash(99)),
            Error::<Test>::CannotSelfAttest
        );
    });
}

#[test]
fn destruction_finalized_with_enough_attestations() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);
        register_and_activate(1, key_hash);

        for i in 2..=5 {
            register_and_activate(i, create_key_hash(i as u8));
        }

        assert_ok!(Lifecycle::initiate_destruction(
            RuntimeOrigin::signed(1),
            DestructionReason::OwnerRequest
        ));

        let actor = account_to_actor(1);

        for i in 2..=4 {
            assert_ok!(Lifecycle::attest_destruction(
                RuntimeOrigin::signed(i),
                actor,
                create_key_hash((i + 10) as u8)
            ));
        }

        let lifecycle = Lifecycle::actors(actor).expect("actor should exist");
        assert_eq!(lifecycle.status, ActorStatus::Destroyed);
        assert_eq!(lifecycle.key_status, KeyStatus::Destroyed);
    });
}

#[test]
fn cancel_destruction_success() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);
        register_and_activate(1, key_hash);

        assert_ok!(Lifecycle::initiate_destruction(
            RuntimeOrigin::signed(1),
            DestructionReason::OwnerRequest
        ));

        assert_ok!(Lifecycle::cancel_destruction(RuntimeOrigin::signed(1)));

        let actor = account_to_actor(1);
        assert!(!Lifecycle::is_destruction_pending(actor));

        let lifecycle = Lifecycle::actors(actor).expect("actor should exist");
        assert_eq!(lifecycle.status, ActorStatus::Active);
    });
}

#[test]
fn initiate_rotation_success() {
    new_test_ext().execute_with(|| {
        let old_key = create_key_hash(1);
        let new_key = create_key_hash(2);
        register_and_activate(1, old_key);

        assert_ok!(Lifecycle::initiate_rotation(
            RuntimeOrigin::signed(1),
            new_key
        ));

        let actor = account_to_actor(1);
        let lifecycle = Lifecycle::actors(actor).expect("actor should exist");
        assert_eq!(lifecycle.key_status, KeyStatus::Rotating);

        let rotation = Lifecycle::key_rotations(actor).expect("rotation should exist");
        assert_eq!(rotation.old_key_hash, old_key);
        assert_eq!(rotation.new_key_hash, new_key);
    });
}

#[test]
fn complete_rotation_success() {
    new_test_ext().execute_with(|| {
        let old_key = create_key_hash(1);
        let new_key = create_key_hash(2);
        register_and_activate(1, old_key);

        assert_ok!(Lifecycle::initiate_rotation(
            RuntimeOrigin::signed(1),
            new_key
        ));

        assert_ok!(Lifecycle::complete_rotation(RuntimeOrigin::signed(1)));

        let actor = account_to_actor(1);
        let lifecycle = Lifecycle::actors(actor).expect("actor should exist");
        assert_eq!(lifecycle.key_hash, new_key);
        assert_eq!(lifecycle.key_status, KeyStatus::Active);
    });
}

#[test]
fn is_key_valid() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);
        register_and_activate(1, key_hash);

        let actor = account_to_actor(1);
        assert!(Lifecycle::is_key_valid(actor, &key_hash));
        assert!(!Lifecycle::is_key_valid(actor, &create_key_hash(99)));
    });
}

#[test]
fn destruction_reasons() {
    new_test_ext().execute_with(|| {
        let reasons = [
            DestructionReason::OwnerRequest,
            DestructionReason::SecurityBreach,
            DestructionReason::Expiration,
            DestructionReason::ProtocolViolation,
            DestructionReason::Administrative,
        ];

        for (i, reason) in reasons.iter().enumerate() {
            let key_hash = create_key_hash((i + 1) as u8);
            register_and_activate((i + 1) as u64, key_hash);

            assert_ok!(Lifecycle::initiate_destruction(
                RuntimeOrigin::signed((i + 1) as u64),
                *reason
            ));

            let actor = account_to_actor((i + 1) as u64);
            let request = Lifecycle::destruction_requests(actor).expect("request should exist");
            assert_eq!(request.reason, *reason);
        }
    });
}

#[test]
fn actor_counts() {
    new_test_ext().execute_with(|| {
        for i in 1..=5 {
            register_and_activate(i, create_key_hash(i as u8));
        }

        assert_eq!(Lifecycle::total_actors(), 5);
        assert_eq!(Lifecycle::total_active(), 5);
    });
}

#[test]
fn genesis_initializes_correctly() {
    new_test_ext().execute_with(|| {
        assert_eq!(Lifecycle::actor_count(), 0);
        assert_eq!(Lifecycle::active_actors(), 0);
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        System::reset_events();

        let key_hash = create_key_hash(1);
        assert_ok!(Lifecycle::register_actor(
            RuntimeOrigin::signed(1),
            key_hash
        ));

        let events = System::events();
        assert!(events.iter().any(|e| matches!(
            &e.event,
            RuntimeEvent::Lifecycle(Event::ActorRegistered { .. })
        )));
    });
}

#[test]
fn cannot_double_attest() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);
        register_and_activate(1, key_hash);
        register_and_activate(2, create_key_hash(2));

        assert_ok!(Lifecycle::initiate_destruction(
            RuntimeOrigin::signed(1),
            DestructionReason::OwnerRequest
        ));

        let actor = account_to_actor(1);
        assert_ok!(Lifecycle::attest_destruction(
            RuntimeOrigin::signed(2),
            actor,
            create_key_hash(99)
        ));

        assert_noop!(
            Lifecycle::attest_destruction(RuntimeOrigin::signed(2), actor, create_key_hash(99)),
            Error::<Test>::AlreadyAttested
        );
    });
}

#[test]
fn suspended_actor_can_initiate_destruction() {
    new_test_ext().execute_with(|| {
        let key_hash = create_key_hash(1);
        register_and_activate(1, key_hash);

        let actor = account_to_actor(1);
        assert_ok!(Lifecycle::suspend_actor(RuntimeOrigin::root(), actor));

        assert_ok!(Lifecycle::initiate_destruction(
            RuntimeOrigin::signed(1),
            DestructionReason::OwnerRequest
        ));

        assert!(Lifecycle::is_destruction_pending(actor));
    });
}
