#![allow(clippy::disallowed_macros)]

use crate::{
    self as pallet_semantic, DiscoveryCriteria, DiscoveryRequestId, DiscoveryStatus, Error, Event,
    RelationshipId, RelationshipStatus, RelationshipType,
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
        Semantic: pallet_semantic,
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
    pub const MaxRelationshipsPerActor: u32 = 50;
    pub const MaxDiscoveryResults: u32 = 100;
    pub const DiscoveryRateLimitBlocks: u64 = 10;
    pub const RelationshipExpiryBlocks: u64 = 1000;
    pub const MaxTrustLevel: u8 = 100;
}

impl pallet_semantic::Config for Test {
    type WeightInfo = ();
    type MaxRelationshipsPerActor = MaxRelationshipsPerActor;
    type MaxDiscoveryResults = MaxDiscoveryResults;
    type DiscoveryRateLimitBlocks = DiscoveryRateLimitBlocks;
    type RelationshipExpiryBlocks = RelationshipExpiryBlocks;
    type MaxTrustLevel = MaxTrustLevel;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_semantic::GenesisConfig::<Test> {
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
fn create_relationship_success() {
    new_test_ext().execute_with(|| {
        let to_actor = account_to_actor(2);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor,
            RelationshipType::Trust,
            50,
            None,
            false
        ));

        let relationship_id = RelationshipId::new(0);
        let relationship =
            Semantic::relationships(relationship_id).expect("relationship should exist");

        assert_eq!(relationship.to_actor, to_actor);
        assert_eq!(relationship.relationship_type, RelationshipType::Trust);
        assert_eq!(relationship.trust_level, 50);
        assert_eq!(relationship.status, RelationshipStatus::Active);
    });
}

#[test]
fn create_bidirectional_relationship_pending() {
    new_test_ext().execute_with(|| {
        let to_actor = account_to_actor(2);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor,
            RelationshipType::Collaborate,
            75,
            None,
            true
        ));

        let relationship_id = RelationshipId::new(0);
        let relationship =
            Semantic::relationships(relationship_id).expect("relationship should exist");

        assert_eq!(relationship.status, RelationshipStatus::Pending);
        assert!(relationship.bidirectional);
    });
}

#[test]
fn self_relationship_prevented() {
    new_test_ext().execute_with(|| {
        let self_actor = account_to_actor(1);

        assert_noop!(
            Semantic::create_relationship(
                RuntimeOrigin::signed(1),
                self_actor,
                RelationshipType::Trust,
                50,
                None,
                false
            ),
            Error::<Test>::SelfRelationship
        );
    });
}

#[test]
fn duplicate_relationship_prevented() {
    new_test_ext().execute_with(|| {
        let to_actor = account_to_actor(2);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor,
            RelationshipType::Trust,
            50,
            None,
            false
        ));

        assert_noop!(
            Semantic::create_relationship(
                RuntimeOrigin::signed(1),
                to_actor,
                RelationshipType::Follow,
                25,
                None,
                false
            ),
            Error::<Test>::RelationshipAlreadyExists
        );
    });
}

#[test]
fn invalid_trust_level_rejected() {
    new_test_ext().execute_with(|| {
        let to_actor = account_to_actor(2);

        assert_noop!(
            Semantic::create_relationship(
                RuntimeOrigin::signed(1),
                to_actor,
                RelationshipType::Trust,
                101,
                None,
                false
            ),
            Error::<Test>::InvalidTrustLevel
        );
    });
}

#[test]
fn accept_relationship_success() {
    new_test_ext().execute_with(|| {
        let to_actor = account_to_actor(2);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor,
            RelationshipType::Collaborate,
            75,
            None,
            true
        ));

        let relationship_id = RelationshipId::new(0);

        assert_ok!(Semantic::accept_relationship(
            RuntimeOrigin::signed(2),
            relationship_id
        ));

        let relationship =
            Semantic::relationships(relationship_id).expect("relationship should exist");
        assert_eq!(relationship.status, RelationshipStatus::Active);
    });
}

#[test]
fn accept_relationship_not_authorized() {
    new_test_ext().execute_with(|| {
        let to_actor = account_to_actor(2);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor,
            RelationshipType::Collaborate,
            75,
            None,
            true
        ));

        let relationship_id = RelationshipId::new(0);

        assert_noop!(
            Semantic::accept_relationship(RuntimeOrigin::signed(3), relationship_id),
            Error::<Test>::NotAuthorized
        );
    });
}

#[test]
fn revoke_relationship_success() {
    new_test_ext().execute_with(|| {
        let to_actor = account_to_actor(2);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor,
            RelationshipType::Trust,
            50,
            None,
            false
        ));

        let relationship_id = RelationshipId::new(0);

        assert_ok!(Semantic::revoke_relationship(
            RuntimeOrigin::signed(1),
            relationship_id
        ));

        let relationship =
            Semantic::relationships(relationship_id).expect("relationship should exist");
        assert_eq!(relationship.status, RelationshipStatus::Revoked);
    });
}

#[test]
fn update_trust_level_success() {
    new_test_ext().execute_with(|| {
        let to_actor = account_to_actor(2);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor,
            RelationshipType::Trust,
            50,
            None,
            false
        ));

        let relationship_id = RelationshipId::new(0);

        assert_ok!(Semantic::update_trust_level(
            RuntimeOrigin::signed(1),
            relationship_id,
            75
        ));

        let relationship =
            Semantic::relationships(relationship_id).expect("relationship should exist");
        assert_eq!(relationship.trust_level, 75);
    });
}

#[test]
fn request_discovery_success() {
    new_test_ext().execute_with(|| {
        let criteria = DiscoveryCriteria::default();

        assert_ok!(Semantic::request_discovery(
            RuntimeOrigin::signed(1),
            criteria
        ));

        let request_id = DiscoveryRequestId::new(0);
        let request = Semantic::discovery_requests(request_id).expect("request should exist");

        assert_eq!(request.status, DiscoveryStatus::Pending);
        assert_eq!(Semantic::get_pending_discovery_count(), 1);
    });
}

#[test]
fn discovery_rate_limited() {
    new_test_ext().execute_with(|| {
        let criteria = DiscoveryCriteria::default();

        assert_ok!(Semantic::request_discovery(
            RuntimeOrigin::signed(1),
            criteria.clone()
        ));

        assert_noop!(
            Semantic::request_discovery(RuntimeOrigin::signed(1), criteria.clone()),
            Error::<Test>::DiscoveryRateLimited
        );

        System::set_block_number(15);

        assert_ok!(Semantic::request_discovery(
            RuntimeOrigin::signed(1),
            criteria
        ));
    });
}

#[test]
fn update_profile_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(Semantic::update_profile(RuntimeOrigin::signed(1), false));

        let actor = account_to_actor(1);
        let profile = Semantic::semantic_profiles(actor).expect("profile should exist");

        assert!(!profile.discovery_enabled);
    });
}

#[test]
fn complete_discovery_success() {
    new_test_ext().execute_with(|| {
        let criteria = DiscoveryCriteria::default();

        assert_ok!(Semantic::request_discovery(
            RuntimeOrigin::signed(1),
            criteria
        ));

        let request_id = DiscoveryRequestId::new(0);

        assert_ok!(Semantic::complete_discovery(
            RuntimeOrigin::root(),
            request_id,
            5
        ));

        let request = Semantic::discovery_requests(request_id).expect("request should exist");
        assert_eq!(request.status, DiscoveryStatus::Completed);
        assert_eq!(request.results_count, 5);
        assert_eq!(Semantic::get_pending_discovery_count(), 0);
    });
}

#[test]
fn relationship_expires() {
    new_test_ext().execute_with(|| {
        let to_actor = account_to_actor(2);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor,
            RelationshipType::Trust,
            50,
            Some(10),
            false
        ));

        let relationship_id = RelationshipId::new(0);

        System::set_block_number(10);
        Semantic::on_initialize(10);

        let relationship =
            Semantic::relationships(relationship_id).expect("relationship should exist");
        assert_eq!(relationship.status, RelationshipStatus::Expired);
    });
}

#[test]
fn has_relationship_helper() {
    new_test_ext().execute_with(|| {
        let from_actor = account_to_actor(1);
        let to_actor = account_to_actor(2);

        assert!(!Semantic::has_relationship(from_actor, to_actor));

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor,
            RelationshipType::Trust,
            50,
            None,
            false
        ));

        assert!(Semantic::has_relationship(from_actor, to_actor));
        assert!(!Semantic::has_relationship(to_actor, from_actor));
    });
}

#[test]
fn get_trust_level_helper() {
    new_test_ext().execute_with(|| {
        let from_actor = account_to_actor(1);
        let to_actor = account_to_actor(2);

        assert_eq!(Semantic::get_trust_level(from_actor, to_actor), None);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor,
            RelationshipType::Trust,
            75,
            None,
            false
        ));

        assert_eq!(Semantic::get_trust_level(from_actor, to_actor), Some(75));
    });
}

#[test]
fn can_discover_helper() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        assert!(Semantic::can_discover(actor));

        assert_ok!(Semantic::request_discovery(
            RuntimeOrigin::signed(1),
            DiscoveryCriteria::default()
        ));

        assert!(!Semantic::can_discover(actor));

        System::set_block_number(15);

        assert!(Semantic::can_discover(actor));
    });
}

#[test]
fn get_actor_relationships_helper() {
    new_test_ext().execute_with(|| {
        let actor1 = account_to_actor(1);
        let to_actor2 = account_to_actor(2);
        let to_actor3 = account_to_actor(3);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor2,
            RelationshipType::Trust,
            50,
            None,
            false
        ));

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor3,
            RelationshipType::Follow,
            25,
            None,
            false
        ));

        let relationships = Semantic::get_actor_relationships(actor1);
        assert_eq!(relationships.len(), 2);
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        let from_actor = account_to_actor(1);
        let to_actor = account_to_actor(2);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor,
            RelationshipType::Trust,
            50,
            None,
            false
        ));

        System::assert_has_event(RuntimeEvent::Semantic(Event::RelationshipCreated {
            relationship_id: RelationshipId::new(0),
            from_actor,
            to_actor,
            relationship_type: RelationshipType::Trust,
        }));
    });
}

#[test]
fn profile_relationship_count_tracking() {
    new_test_ext().execute_with(|| {
        let actor1 = account_to_actor(1);
        let to_actor2 = account_to_actor(2);

        assert_ok!(Semantic::create_relationship(
            RuntimeOrigin::signed(1),
            to_actor2,
            RelationshipType::Trust,
            50,
            None,
            false
        ));

        let profile = Semantic::semantic_profiles(actor1).expect("profile should exist");
        assert_eq!(profile.total_relationships, 1);

        assert_ok!(Semantic::revoke_relationship(
            RuntimeOrigin::signed(1),
            RelationshipId::new(0)
        ));

        let profile = Semantic::semantic_profiles(actor1).expect("profile should exist");
        assert_eq!(profile.total_relationships, 0);
    });
}

#[test]
fn genesis_initializes_counts() {
    new_test_ext().execute_with(|| {
        assert_eq!(Semantic::relationship_count(), 0);
        assert_eq!(Semantic::discovery_count(), 0);
    });
}
