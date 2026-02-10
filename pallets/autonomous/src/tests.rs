#![allow(clippy::disallowed_macros)]

use crate::{
    self as pallet_autonomous, AutonomousStatus, BehaviorId, BehaviorType, Error, Event,
    PatternClassification, PatternId,
};
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
        Autonomous: pallet_autonomous,
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
    pub const PatternThreshold: u32 = 3;
    pub const MaxBehaviorsPerActor: u32 = 100;
    pub const MaxPatterns: u32 = 50;
    pub const BehaviorExpiryBlocks: u64 = 1000;
    pub const ScoreIncreasePerMatch: u8 = 10;
}

impl pallet_autonomous::Config for Test {
    type WeightInfo = ();
    type PatternThreshold = PatternThreshold;
    type MaxBehaviorsPerActor = MaxBehaviorsPerActor;
    type MaxPatterns = MaxPatterns;
    type BehaviorExpiryBlocks = BehaviorExpiryBlocks;
    type ScoreIncreasePerMatch = ScoreIncreasePerMatch;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_autonomous::GenesisConfig::<Test> {
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
fn record_behavior_creates_profile() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        assert_ok!(Autonomous::record_behavior(
            RuntimeOrigin::signed(1),
            actor,
            BehaviorType::PresencePattern,
            H256([1u8; 32])
        ));

        let profile = Autonomous::actor_profiles(actor).expect("profile should exist");
        assert_eq!(profile.actor, actor);
        assert_eq!(profile.behavior_count, 1);
        assert_eq!(profile.status, AutonomousStatus::Unknown);
    });
}

#[test]
fn record_behavior_increments_count() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        for i in 0..5 {
            assert_ok!(Autonomous::record_behavior(
                RuntimeOrigin::signed(1),
                actor,
                BehaviorType::PresencePattern,
                H256([i as u8; 32])
            ));
        }

        let profile = Autonomous::actor_profiles(actor).expect("profile should exist");
        assert_eq!(profile.behavior_count, 5);
        assert_eq!(Autonomous::behavior_count_per_actor(actor), 5);
    });
}

#[test]
fn max_behaviors_enforced() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        for i in 0..100 {
            assert_ok!(Autonomous::record_behavior(
                RuntimeOrigin::signed(1),
                actor,
                BehaviorType::PresencePattern,
                H256([i as u8; 32])
            ));
        }

        assert_noop!(
            Autonomous::record_behavior(
                RuntimeOrigin::signed(1),
                actor,
                BehaviorType::PresencePattern,
                H256([101u8; 32])
            ),
            Error::<Test>::MaxBehaviorsReached
        );
    });
}

#[test]
fn register_pattern_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(Autonomous::register_pattern(
            RuntimeOrigin::root(),
            BehaviorType::PresencePattern,
            H256([1u8; 32]),
            PatternClassification::Normal
        ));

        let pattern_id = PatternId::new(0);
        let pattern = Autonomous::patterns(pattern_id).expect("pattern should exist");
        assert_eq!(pattern.behavior_type, BehaviorType::PresencePattern);
        assert_eq!(pattern.classification, PatternClassification::Normal);
        assert!(!pattern.threshold_met);
        assert_eq!(Autonomous::get_active_patterns(), 1);
    });
}

#[test]
fn register_pattern_duplicate_prevented() {
    new_test_ext().execute_with(|| {
        assert_ok!(Autonomous::register_pattern(
            RuntimeOrigin::root(),
            BehaviorType::PresencePattern,
            H256([1u8; 32]),
            PatternClassification::Normal
        ));

        assert_noop!(
            Autonomous::register_pattern(
                RuntimeOrigin::root(),
                BehaviorType::PresencePattern,
                H256([1u8; 32]),
                PatternClassification::Automated
            ),
            Error::<Test>::PatternAlreadyExists
        );
    });
}

#[test]
fn classify_pattern_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(Autonomous::register_pattern(
            RuntimeOrigin::root(),
            BehaviorType::PresencePattern,
            H256([1u8; 32]),
            PatternClassification::Normal
        ));

        let pattern_id = PatternId::new(0);

        assert_ok!(Autonomous::classify_pattern(
            RuntimeOrigin::root(),
            pattern_id,
            PatternClassification::Automated,
            85
        ));

        let pattern = Autonomous::patterns(pattern_id).expect("pattern should exist");
        assert_eq!(pattern.classification, PatternClassification::Automated);
        assert_eq!(pattern.confidence_score, 85);
    });
}

#[test]
fn classify_pattern_invalid_score() {
    new_test_ext().execute_with(|| {
        assert_ok!(Autonomous::register_pattern(
            RuntimeOrigin::root(),
            BehaviorType::PresencePattern,
            H256([1u8; 32]),
            PatternClassification::Normal
        ));

        assert_noop!(
            Autonomous::classify_pattern(
                RuntimeOrigin::root(),
                PatternId::new(0),
                PatternClassification::Automated,
                101
            ),
            Error::<Test>::InvalidConfidenceScore
        );
    });
}

#[test]
fn update_status_success() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        assert_ok!(Autonomous::create_profile(RuntimeOrigin::signed(1), actor));

        assert_ok!(Autonomous::update_status(
            RuntimeOrigin::root(),
            actor,
            AutonomousStatus::Confirmed
        ));

        let profile = Autonomous::actor_profiles(actor).expect("profile should exist");
        assert_eq!(profile.status, AutonomousStatus::Confirmed);
    });
}

#[test]
fn flag_actor_success() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        assert_ok!(Autonomous::create_profile(RuntimeOrigin::signed(1), actor));

        assert_ok!(Autonomous::flag_actor(
            RuntimeOrigin::root(),
            actor,
            H256([1u8; 32])
        ));

        let profile = Autonomous::actor_profiles(actor).expect("profile should exist");
        assert_eq!(profile.status, AutonomousStatus::Flagged);
        assert_eq!(profile.flag_count, 1);
    });
}

#[test]
fn cannot_flag_already_flagged() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        assert_ok!(Autonomous::create_profile(RuntimeOrigin::signed(1), actor));
        assert_ok!(Autonomous::flag_actor(
            RuntimeOrigin::root(),
            actor,
            H256([1u8; 32])
        ));

        assert_noop!(
            Autonomous::flag_actor(RuntimeOrigin::root(), actor, H256([2u8; 32])),
            Error::<Test>::CannotFlagActor
        );
    });
}

#[test]
fn pattern_threshold_met_after_occurrences() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);
        let signature = H256([1u8; 32]);

        assert_ok!(Autonomous::register_pattern(
            RuntimeOrigin::root(),
            BehaviorType::PresencePattern,
            signature,
            PatternClassification::Normal
        ));

        let pattern_id = PatternId::new(0);

        for _ in 0..3 {
            assert_ok!(Autonomous::record_behavior(
                RuntimeOrigin::signed(1),
                actor,
                BehaviorType::PresencePattern,
                H256([1u8; 32])
            ));

            assert_ok!(Autonomous::match_behavior(
                RuntimeOrigin::root(),
                BehaviorId::new(Autonomous::behavior_count() - 1),
                actor,
                pattern_id
            ));
        }

        assert!(Autonomous::pattern_threshold_met(pattern_id));
        assert_eq!(Autonomous::get_pattern_occurrences(pattern_id), 3);
    });
}

#[test]
fn automation_score_increases_with_matches() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);
        let signature = H256([1u8; 32]);

        assert_ok!(Autonomous::register_pattern(
            RuntimeOrigin::root(),
            BehaviorType::PresencePattern,
            signature,
            PatternClassification::Normal
        ));

        let pattern_id = PatternId::new(0);

        for i in 0..5 {
            assert_ok!(Autonomous::record_behavior(
                RuntimeOrigin::signed(1),
                actor,
                BehaviorType::PresencePattern,
                H256([i as u8; 32])
            ));

            assert_ok!(Autonomous::match_behavior(
                RuntimeOrigin::root(),
                BehaviorId::new(Autonomous::behavior_count() - 1),
                actor,
                pattern_id
            ));
        }

        assert_eq!(Autonomous::get_automation_score(actor), 50);
    });
}

#[test]
fn status_transitions_based_on_score() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);
        let signature = H256([1u8; 32]);

        assert_ok!(Autonomous::register_pattern(
            RuntimeOrigin::root(),
            BehaviorType::PresencePattern,
            signature,
            PatternClassification::Normal
        ));

        let pattern_id = PatternId::new(0);

        for i in 0..6 {
            assert_ok!(Autonomous::record_behavior(
                RuntimeOrigin::signed(1),
                actor,
                BehaviorType::PresencePattern,
                H256([i as u8; 32])
            ));

            assert_ok!(Autonomous::match_behavior(
                RuntimeOrigin::root(),
                BehaviorId::new(Autonomous::behavior_count() - 1),
                actor,
                pattern_id
            ));
        }

        let profile = Autonomous::actor_profiles(actor).expect("profile should exist");
        assert_eq!(profile.automation_score, 60);
        assert_eq!(profile.status, AutonomousStatus::Confirmed);
    });
}

#[test]
fn is_autonomous_helper() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        assert_ok!(Autonomous::create_profile(RuntimeOrigin::signed(1), actor));
        assert!(!Autonomous::is_autonomous(actor));

        assert_ok!(Autonomous::update_status(
            RuntimeOrigin::root(),
            actor,
            AutonomousStatus::Confirmed
        ));

        assert!(Autonomous::is_autonomous(actor));
    });
}

#[test]
fn create_profile_duplicate_prevented() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        assert_ok!(Autonomous::create_profile(RuntimeOrigin::signed(1), actor));

        assert_noop!(
            Autonomous::create_profile(RuntimeOrigin::signed(1), actor),
            Error::<Test>::ProfileAlreadyExists
        );
    });
}

#[test]
fn get_actor_behaviors_helper() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        for i in 0..3 {
            assert_ok!(Autonomous::record_behavior(
                RuntimeOrigin::signed(1),
                actor,
                BehaviorType::PresencePattern,
                H256([i as u8; 32])
            ));
        }

        let behaviors = Autonomous::get_actor_behaviors(actor);
        assert_eq!(behaviors.len(), 3);
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        assert_ok!(Autonomous::record_behavior(
            RuntimeOrigin::signed(1),
            actor,
            BehaviorType::PresencePattern,
            H256([1u8; 32])
        ));

        System::assert_has_event(RuntimeEvent::Autonomous(Event::ProfileCreated { actor }));

        System::assert_has_event(RuntimeEvent::Autonomous(Event::BehaviorRecorded {
            behavior_id: BehaviorId::new(0),
            actor,
            behavior_type: BehaviorType::PresencePattern,
        }));
    });
}

#[test]
fn genesis_initializes_counts() {
    new_test_ext().execute_with(|| {
        assert_eq!(Autonomous::behavior_count(), 0);
        assert_eq!(Autonomous::pattern_count(), 0);
        assert_eq!(Autonomous::get_active_patterns(), 0);
    });
}

#[test]
fn max_patterns_enforced() {
    new_test_ext().execute_with(|| {
        for i in 0..50 {
            assert_ok!(Autonomous::register_pattern(
                RuntimeOrigin::root(),
                BehaviorType::PresencePattern,
                H256([i as u8; 32]),
                PatternClassification::Normal
            ));
        }

        assert_noop!(
            Autonomous::register_pattern(
                RuntimeOrigin::root(),
                BehaviorType::PresencePattern,
                H256([51u8; 32]),
                PatternClassification::Normal
            ),
            Error::<Test>::MaxPatternsReached
        );
    });
}

#[test]
fn match_behavior_nonexistent_pattern() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        assert_ok!(Autonomous::record_behavior(
            RuntimeOrigin::signed(1),
            actor,
            BehaviorType::PresencePattern,
            H256([1u8; 32])
        ));

        assert_noop!(
            Autonomous::match_behavior(
                RuntimeOrigin::root(),
                BehaviorId::new(0),
                actor,
                PatternId::new(999)
            ),
            Error::<Test>::PatternNotFound
        );
    });
}

#[test]
fn match_behavior_nonexistent_behavior() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(1);

        assert_ok!(Autonomous::register_pattern(
            RuntimeOrigin::root(),
            BehaviorType::PresencePattern,
            H256([1u8; 32]),
            PatternClassification::Normal
        ));

        assert_noop!(
            Autonomous::match_behavior(
                RuntimeOrigin::root(),
                BehaviorId::new(999),
                actor,
                PatternId::new(0)
            ),
            Error::<Test>::BehaviorNotFound
        );
    });
}
