#![allow(clippy::disallowed_macros)]

use crate::{self as pallet_epoch, Error, Event};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, Hooks},
};
use frame_system as system;
use seveny_primitives::crypto::{hash_with_domain, DOMAIN_VRF_EPOCH};
use seveny_primitives::types::{EpochId, EpochState};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Epoch: pallet_epoch,
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
    pub const EpochDuration: u64 = 100;
    pub const MinEpochDuration: u64 = 10;
    pub const MaxEpochDuration: u64 = 1000;
    pub const GracePeriod: u64 = 10;
}

impl pallet_epoch::Config for Test {
    type WeightInfo = ();
    type EpochDuration = EpochDuration;
    type MinEpochDuration = MinEpochDuration;
    type MaxEpochDuration = MaxEpochDuration;
    type GracePeriod = GracePeriod;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_epoch::GenesisConfig::<Test> {
        initial_epoch_duration: 100,
        initial_grace_period: 10,
        auto_transition: true,
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("genesis build failed");

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
        Epoch::on_initialize(System::block_number());
    }
}

#[test]
fn genesis_creates_first_epoch() {
    new_test_ext().execute_with(|| {
        let current = Epoch::current_epoch();
        assert_eq!(current, EpochId::new(1));

        let metadata = Epoch::epoch_info(current).expect("epoch should exist");
        assert_eq!(metadata.state, EpochState::Active);
        assert_eq!(metadata.start_block, 1);
        assert_eq!(metadata.end_block, 101);
    });
}

#[test]
fn invariant_inv14_bounded_epochs_have_defined_start_end() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);
        let metadata = Epoch::epoch_info(epoch_id).expect("epoch should exist");

        assert!(metadata.start_block < metadata.end_block);
        assert_eq!(metadata.end_block - metadata.start_block, 100);
    });
}

#[test]
fn invariant_inv15_sequential_epochs_no_gaps() {
    new_test_ext().execute_with(|| {
        run_to_block(102);

        let epoch2 = Epoch::epoch_info(EpochId::new(2));
        assert!(epoch2.is_some());

        assert_noop!(
            Epoch::start_epoch(RuntimeOrigin::root(), EpochId::new(5)),
            Error::<Test>::EpochNotFound
        );
    });
}

#[test]
fn invariant_inv16_actor_epoch_binding() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        assert_ok!(Epoch::register_participant(
            RuntimeOrigin::signed(1),
            epoch_id
        ));

        assert!(Epoch::is_participant(epoch_id, &1));
        assert!(!Epoch::is_participant(epoch_id, &2));

        assert_noop!(
            Epoch::register_participant(RuntimeOrigin::signed(1), epoch_id),
            Error::<Test>::ParticipantAlreadyRegistered
        );
    });
}

#[test]
fn invariant_inv17_epoch_immutability_past_epochs() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        assert_ok!(Epoch::close_epoch(RuntimeOrigin::root(), epoch_id));

        run_to_block(120);

        assert_ok!(Epoch::finalize_epoch(RuntimeOrigin::root(), epoch_id));

        let metadata = Epoch::epoch_info(epoch_id).expect("epoch should exist");
        assert_eq!(metadata.state, EpochState::Finalized);

        assert_noop!(
            Epoch::force_transition(RuntimeOrigin::root(), epoch_id, EpochState::Active),
            Error::<Test>::EpochImmutable
        );
    });
}

#[test]
fn invariant_inv18_graceful_transition() {
    new_test_ext().execute_with(|| {
        let epoch1 = EpochId::new(1);

        let metadata = Epoch::epoch_info(epoch1).expect("epoch should exist");
        assert_eq!(metadata.state, EpochState::Active);

        run_to_block(101);

        let metadata = Epoch::epoch_info(epoch1).expect("epoch should exist");
        assert_eq!(metadata.state, EpochState::Closed);

        run_to_block(112);

        let epoch2_metadata = Epoch::epoch_info(EpochId::new(2));
        assert!(epoch2_metadata.is_some());

        let current = Epoch::current_epoch();
        assert_eq!(current, EpochId::new(2));
    });
}

#[test]
fn epoch_state_transitions_follow_order() {
    new_test_ext().execute_with(|| {
        assert!(EpochState::Scheduled.can_transition_to(&EpochState::Active));
        assert!(EpochState::Active.can_transition_to(&EpochState::Closed));
        assert!(EpochState::Closed.can_transition_to(&EpochState::Finalized));

        assert!(!EpochState::Active.can_transition_to(&EpochState::Scheduled));
        assert!(!EpochState::Closed.can_transition_to(&EpochState::Active));
        assert!(!EpochState::Finalized.can_transition_to(&EpochState::Closed));
    });
}

#[test]
fn schedule_epoch_success() {
    new_test_ext().execute_with(|| {
        run_to_block(200);

        assert_ok!(Epoch::schedule_epoch(RuntimeOrigin::root(), 300, 100));

        let epoch_id = EpochId::new(3);
        let metadata = Epoch::epoch_info(epoch_id).expect("epoch should exist");

        assert_eq!(metadata.state, EpochState::Scheduled);
        assert_eq!(metadata.start_block, 300);
        assert_eq!(metadata.end_block, 400);
    });
}

#[test]
fn schedule_epoch_invalid_duration() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Epoch::schedule_epoch(RuntimeOrigin::root(), 200, 5),
            Error::<Test>::InvalidEpochDuration
        );

        assert_noop!(
            Epoch::schedule_epoch(RuntimeOrigin::root(), 200, 2000),
            Error::<Test>::InvalidEpochDuration
        );
    });
}

#[test]
fn manual_epoch_lifecycle() {
    new_test_ext().execute_with(|| {
        let schedule = pallet_epoch::EpochScheduleConfig {
            duration: 100,
            grace_period: 10,
            auto_transition: false,
        };
        pallet_epoch::EpochSchedule::<Test>::put(schedule);

        assert_ok!(Epoch::schedule_epoch(RuntimeOrigin::root(), 200, 100));

        let epoch_id = EpochId::new(2);

        assert_ok!(Epoch::close_epoch(RuntimeOrigin::root(), EpochId::new(1)));

        assert_ok!(Epoch::start_epoch(RuntimeOrigin::root(), epoch_id));

        let metadata = Epoch::epoch_info(epoch_id).expect("epoch should exist");
        assert_eq!(metadata.state, EpochState::Active);

        assert_ok!(Epoch::close_epoch(RuntimeOrigin::root(), epoch_id));

        let metadata = Epoch::epoch_info(epoch_id).expect("epoch should exist");
        assert_eq!(metadata.state, EpochState::Closed);

        run_to_block(320);

        assert_ok!(Epoch::finalize_epoch(RuntimeOrigin::root(), epoch_id));

        let metadata = Epoch::epoch_info(epoch_id).expect("epoch should exist");
        assert_eq!(metadata.state, EpochState::Finalized);
    });
}

#[test]
fn cannot_finalize_before_grace_period() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        assert_ok!(Epoch::close_epoch(RuntimeOrigin::root(), epoch_id));

        assert_noop!(
            Epoch::finalize_epoch(RuntimeOrigin::root(), epoch_id),
            Error::<Test>::GracePeriodNotElapsed
        );

        run_to_block(112);

        assert_ok!(Epoch::finalize_epoch(RuntimeOrigin::root(), epoch_id));
    });
}

#[test]
fn update_schedule_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(Epoch::update_schedule(
            RuntimeOrigin::root(),
            200,
            20,
            false
        ));

        let schedule = Epoch::epoch_schedule();
        assert_eq!(schedule.duration, 200);
        assert_eq!(schedule.grace_period, 20);
        assert!(!schedule.auto_transition);
    });
}

#[test]
fn update_schedule_invalid() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Epoch::update_schedule(RuntimeOrigin::root(), 5, 10, true),
            Error::<Test>::InvalidScheduleConfig
        );
    });
}

#[test]
fn force_transition_success() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        // Active -> Closed is valid
        assert_ok!(Epoch::force_transition(
            RuntimeOrigin::root(),
            epoch_id,
            EpochState::Closed
        ));

        let metadata = Epoch::epoch_info(epoch_id).expect("epoch should exist");
        assert_eq!(metadata.state, EpochState::Closed);

        // Closed -> Finalized requires grace period
        run_to_block(112);
        assert_ok!(Epoch::force_transition(
            RuntimeOrigin::root(),
            epoch_id,
            EpochState::Finalized
        ));

        let metadata = Epoch::epoch_info(epoch_id).expect("epoch should exist");
        assert_eq!(metadata.state, EpochState::Finalized);
    });
}

#[test]
fn force_transition_invalid() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        // Cannot go backwards
        assert_noop!(
            Epoch::force_transition(RuntimeOrigin::root(), epoch_id, EpochState::Scheduled),
            Error::<Test>::InvalidEpochTransition
        );
    });
}

#[test]
fn force_transition_finalize_requires_grace_period() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        // Close the epoch first
        assert_ok!(Epoch::force_transition(
            RuntimeOrigin::root(),
            epoch_id,
            EpochState::Closed
        ));

        // Try to finalize immediately - should fail due to grace period
        assert_noop!(
            Epoch::force_transition(RuntimeOrigin::root(), epoch_id, EpochState::Finalized),
            Error::<Test>::GracePeriodNotElapsed
        );
    });
}

#[test]
fn participant_count_tracking() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        assert_ok!(Epoch::register_participant(
            RuntimeOrigin::signed(1),
            epoch_id
        ));
        assert_ok!(Epoch::register_participant(
            RuntimeOrigin::signed(2),
            epoch_id
        ));
        assert_ok!(Epoch::register_participant(
            RuntimeOrigin::signed(3),
            epoch_id
        ));

        let metadata = Epoch::epoch_info(epoch_id).expect("epoch should exist");
        assert_eq!(metadata.participant_count, 3);
    });
}

#[test]
fn cannot_register_in_non_active_epoch() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        assert_ok!(Epoch::close_epoch(RuntimeOrigin::root(), epoch_id));

        assert_noop!(
            Epoch::register_participant(RuntimeOrigin::signed(1), epoch_id),
            Error::<Test>::EpochNotActive
        );
    });
}

#[test]
fn auto_transition_creates_next_epoch() {
    new_test_ext().execute_with(|| {
        run_to_block(101);

        let epoch2 = Epoch::epoch_info(EpochId::new(2));
        assert!(epoch2.is_some());

        let metadata = epoch2.expect("epoch 2 should exist");
        assert_eq!(metadata.state, EpochState::Scheduled);
    });
}

#[test]
fn last_finalized_epoch_tracking() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        assert!(Epoch::last_finalized_epoch().is_none());

        assert_ok!(Epoch::close_epoch(RuntimeOrigin::root(), epoch_id));
        run_to_block(112);
        assert_ok!(Epoch::finalize_epoch(RuntimeOrigin::root(), epoch_id));

        assert_eq!(Epoch::last_finalized_epoch(), Some(epoch_id));
    });
}

#[test]
fn helper_functions_work() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        assert_eq!(Epoch::get_epoch_state(epoch_id), Some(EpochState::Active));
        assert!(Epoch::is_epoch_active(epoch_id));
        assert!(!Epoch::is_epoch_active(EpochId::new(999)));

        let current_metadata = Epoch::get_current_epoch_metadata();
        assert!(current_metadata.is_some());
        assert_eq!(current_metadata.as_ref().map(|m| m.id), Some(epoch_id));
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        assert_ok!(Epoch::register_participant(
            RuntimeOrigin::signed(1),
            epoch_id
        ));

        System::assert_has_event(RuntimeEvent::Epoch(Event::ParticipantRegistered {
            epoch_id,
            participant: 1,
        }));
    });
}

// =========================================================================
// VRF Epoch Randomness Tests
// =========================================================================

#[test]
fn submit_vrf_in_active_epoch_succeeds() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);
        let vrf_output = H256::repeat_byte(0xAA);
        let vrf_proof = [0u8; 64];

        assert_ok!(Epoch::submit_epoch_vrf(
            RuntimeOrigin::signed(1),
            epoch_id,
            vrf_output,
            vrf_proof,
        ));

        System::assert_has_event(RuntimeEvent::Epoch(Event::EpochVrfSubmitted {
            epoch_id,
            submitter: 1,
        }));
    });
}

#[test]
fn submit_vrf_in_non_active_epoch_fails() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        // Close the epoch so it is no longer Active
        assert_ok!(Epoch::close_epoch(RuntimeOrigin::root(), epoch_id));

        let vrf_output = H256::repeat_byte(0xBB);
        let vrf_proof = [0u8; 64];

        assert_noop!(
            Epoch::submit_epoch_vrf(RuntimeOrigin::signed(1), epoch_id, vrf_output, vrf_proof,),
            Error::<Test>::VrfSubmissionNotActive
        );
    });
}

#[test]
fn submit_vrf_for_wrong_epoch_fails() {
    new_test_ext().execute_with(|| {
        // Current epoch is 1; submitting for epoch 99 should fail
        let wrong_epoch = EpochId::new(99);
        let vrf_output = H256::repeat_byte(0xCC);
        let vrf_proof = [0u8; 64];

        assert_noop!(
            Epoch::submit_epoch_vrf(RuntimeOrigin::signed(1), wrong_epoch, vrf_output, vrf_proof,),
            Error::<Test>::VrfEpochMismatch
        );
    });
}

#[test]
fn vrf_seed_stored_correctly() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);
        let vrf_output = H256::repeat_byte(0xDD);
        let vrf_proof = [0u8; 64];

        // Before submission, no seed exists
        assert!(Epoch::epoch_seed(epoch_id).is_none());

        assert_ok!(Epoch::submit_epoch_vrf(
            RuntimeOrigin::signed(1),
            epoch_id,
            vrf_output,
            vrf_proof,
        ));

        // After submission, seed should be present
        let seed = Epoch::epoch_seed(epoch_id);
        assert!(seed.is_some());

        // Verify the seed is computed deterministically:
        // seed = hash_with_domain(DOMAIN_VRF_EPOCH, vrf_output || prev_seed || epoch_id)
        let prev_seed = H256::zero(); // PreviousEpochSeed default is zero
        let epoch_id_bytes = epoch_id.inner().to_le_bytes();
        let mut data = [0u8; 72];
        data[..32].copy_from_slice(vrf_output.as_bytes());
        data[32..64].copy_from_slice(prev_seed.as_bytes());
        data[64..72].copy_from_slice(&epoch_id_bytes);
        let expected = hash_with_domain(DOMAIN_VRF_EPOCH, &data);

        assert_eq!(seed, Some(expected));
    });
}

#[test]
fn seed_propagates_to_previous_on_finalization() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);
        let vrf_output = H256::repeat_byte(0xEE);
        let vrf_proof = [0u8; 64];

        // Submit a VRF for epoch 1
        assert_ok!(Epoch::submit_epoch_vrf(
            RuntimeOrigin::signed(1),
            epoch_id,
            vrf_output,
            vrf_proof,
        ));

        let seed_epoch1 = Epoch::epoch_seed(epoch_id).expect("seed should exist after submission");

        // PreviousEpochSeed should still be the default (zero) before finalization
        assert_eq!(Epoch::previous_epoch_seed(), H256::zero());

        // Close and finalize epoch 1
        assert_ok!(Epoch::close_epoch(RuntimeOrigin::root(), epoch_id));
        run_to_block(112);
        assert_ok!(Epoch::finalize_epoch(RuntimeOrigin::root(), epoch_id));

        // Now PreviousEpochSeed should have been propagated
        assert_eq!(Epoch::previous_epoch_seed(), seed_epoch1);
    });
}

#[test]
fn multiple_vrf_submissions_mix_correctly() {
    new_test_ext().execute_with(|| {
        let epoch_id = EpochId::new(1);

        let vrf_output_1 = H256::repeat_byte(0x11);
        let vrf_output_2 = H256::repeat_byte(0x22);
        let vrf_proof = [0u8; 64];

        // First submission: uses PreviousEpochSeed (zero default)
        assert_ok!(Epoch::submit_epoch_vrf(
            RuntimeOrigin::signed(1),
            epoch_id,
            vrf_output_1,
            vrf_proof,
        ));

        let seed_after_first =
            Epoch::epoch_seed(epoch_id).expect("seed should exist after first submission");

        // Second submission: should use the seed from the first submission
        // as the prev_seed input, not the PreviousEpochSeed storage value
        assert_ok!(Epoch::submit_epoch_vrf(
            RuntimeOrigin::signed(2),
            epoch_id,
            vrf_output_2,
            vrf_proof,
        ));

        let seed_after_second =
            Epoch::epoch_seed(epoch_id).expect("seed should exist after second submission");

        // The two seeds must differ (different inputs)
        assert_ne!(seed_after_first, seed_after_second);

        // Verify the second seed is computed from the first seed:
        // seed2 = H(DOMAIN_VRF_EPOCH || vrf_output_2 || seed_after_first || epoch_id)
        let epoch_id_bytes = epoch_id.inner().to_le_bytes();
        let mut data = [0u8; 72];
        data[..32].copy_from_slice(vrf_output_2.as_bytes());
        data[32..64].copy_from_slice(seed_after_first.as_bytes());
        data[64..72].copy_from_slice(&epoch_id_bytes);
        let expected = hash_with_domain(DOMAIN_VRF_EPOCH, &data);

        assert_eq!(seed_after_second, expected);
    });
}
