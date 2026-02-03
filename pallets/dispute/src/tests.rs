#![allow(clippy::disallowed_macros)]

use crate::{self as pallet_dispute, DisputeId, DisputeOutcome, DisputeRejectionReason, DisputeStatus, Error, Event};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::ConstU32,
};
use frame_system as system;
use seveny_primitives::types::{ValidatorId, ViolationType};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, Hash, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Dispute: pallet_dispute,
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
    pub const MaxEvidencePerDispute: u32 = 10;
    pub const DisputeResolutionPeriod: u64 = 100;
    pub const MinEvidenceRequired: u32 = 2;
    pub const MaxDisputesPerValidator: u32 = 5;
    pub const MaxOpenDisputes: u32 = 20;
}

impl pallet_dispute::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxEvidencePerDispute = MaxEvidencePerDispute;
    type DisputeResolutionPeriod = DisputeResolutionPeriod;
    type MinEvidenceRequired = MinEvidenceRequired;
    type MaxDisputesPerValidator = MaxDisputesPerValidator;
    type MaxOpenDisputes = MaxOpenDisputes;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_dispute::GenesisConfig::<Test> {
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("genesis build failed");

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn account_to_validator(account: u64) -> ValidatorId {
    let hash = BlakeTwo256::hash_of(&account);
    ValidatorId::from(H256(hash.0))
}

#[test]
fn open_dispute_success() {
    new_test_ext().execute_with(|| {
        let target = account_to_validator(1);

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(2),
            target,
            ViolationType::Minor
        ));

        let dispute_id = DisputeId::new(0);
        let dispute = Dispute::disputes(dispute_id).expect("dispute should exist");

        assert_eq!(dispute.reporter, 2);
        assert_eq!(dispute.target, target);
        assert_eq!(dispute.status, DisputeStatus::Open);
        assert_eq!(dispute.evidence_count, 0);
        assert_eq!(Dispute::dispute_count(), 1);
    });
}

#[test]
fn submit_evidence_success() {
    new_test_ext().execute_with(|| {
        let target = account_to_validator(1);

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(2),
            target,
            ViolationType::Minor
        ));

        let dispute_id = DisputeId::new(0);
        let evidence_hash = H256([1u8; 32]);

        assert_ok!(Dispute::submit_evidence(
            RuntimeOrigin::signed(3),
            dispute_id,
            evidence_hash
        ));

        let dispute = Dispute::disputes(dispute_id).expect("dispute should exist");
        assert_eq!(dispute.evidence_count, 1);
        assert_eq!(dispute.status, DisputeStatus::Open);
    });
}

#[test]
fn submit_evidence_triggers_review() {
    new_test_ext().execute_with(|| {
        let target = account_to_validator(1);

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(2),
            target,
            ViolationType::Minor
        ));

        let dispute_id = DisputeId::new(0);

        assert_ok!(Dispute::submit_evidence(
            RuntimeOrigin::signed(3),
            dispute_id,
            H256([1u8; 32])
        ));

        let dispute = Dispute::disputes(dispute_id).expect("dispute should exist");
        assert_eq!(dispute.status, DisputeStatus::Open);

        assert_ok!(Dispute::submit_evidence(
            RuntimeOrigin::signed(4),
            dispute_id,
            H256([2u8; 32])
        ));

        let dispute = Dispute::disputes(dispute_id).expect("dispute should exist");
        assert_eq!(dispute.status, DisputeStatus::UnderReview);
        assert_eq!(dispute.evidence_count, 2);
    });
}

#[test]
fn submit_evidence_max_reached() {
    new_test_ext().execute_with(|| {
        let target = account_to_validator(1);

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(2),
            target,
            ViolationType::Minor
        ));

        let dispute_id = DisputeId::new(0);

        for i in 0..10 {
            assert_ok!(Dispute::submit_evidence(
                RuntimeOrigin::signed(i + 10),
                dispute_id,
                H256([(i as u8); 32])
            ));
        }

        assert_noop!(
            Dispute::submit_evidence(
                RuntimeOrigin::signed(100),
                dispute_id,
                H256([99u8; 32])
            ),
            Error::<Test>::MaxEvidenceReached
        );
    });
}

#[test]
fn resolve_dispute_success() {
    new_test_ext().execute_with(|| {
        let target = account_to_validator(1);

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(2),
            target,
            ViolationType::Severe
        ));

        let dispute_id = DisputeId::new(0);

        assert_ok!(Dispute::resolve_dispute(
            RuntimeOrigin::root(),
            dispute_id,
            DisputeOutcome::ValidatorSlashed
        ));

        let dispute = Dispute::disputes(dispute_id).expect("dispute should exist");
        assert_eq!(dispute.status, DisputeStatus::Resolved);
        assert_eq!(dispute.outcome, Some(DisputeOutcome::ValidatorSlashed));
        assert!(dispute.resolved_at.is_some());
    });
}

#[test]
fn resolve_dispute_already_resolved() {
    new_test_ext().execute_with(|| {
        let target = account_to_validator(1);

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(2),
            target,
            ViolationType::Minor
        ));

        let dispute_id = DisputeId::new(0);

        assert_ok!(Dispute::resolve_dispute(
            RuntimeOrigin::root(),
            dispute_id,
            DisputeOutcome::ValidatorSlashed
        ));

        assert_noop!(
            Dispute::resolve_dispute(
                RuntimeOrigin::root(),
                dispute_id,
                DisputeOutcome::InsufficientEvidence
            ),
            Error::<Test>::DisputeAlreadyResolved
        );
    });
}

#[test]
fn reject_dispute_success() {
    new_test_ext().execute_with(|| {
        let target = account_to_validator(1);

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(2),
            target,
            ViolationType::Minor
        ));

        let dispute_id = DisputeId::new(0);

        assert_ok!(Dispute::reject_dispute(
            RuntimeOrigin::root(),
            dispute_id,
            DisputeRejectionReason::InsufficientEvidence
        ));

        let dispute = Dispute::disputes(dispute_id).expect("dispute should exist");
        assert_eq!(dispute.status, DisputeStatus::Rejected);
        assert_eq!(dispute.outcome, Some(DisputeOutcome::DisputeRejected));
    });
}

#[test]
fn dispute_not_found() {
    new_test_ext().execute_with(|| {
        let dispute_id = DisputeId::new(999);

        assert_noop!(
            Dispute::submit_evidence(
                RuntimeOrigin::signed(1),
                dispute_id,
                H256([1u8; 32])
            ),
            Error::<Test>::DisputeNotFound
        );

        assert_noop!(
            Dispute::resolve_dispute(
                RuntimeOrigin::root(),
                dispute_id,
                DisputeOutcome::ValidatorSlashed
            ),
            Error::<Test>::DisputeNotFound
        );
    });
}

#[test]
fn disputes_by_validator_tracking() {
    new_test_ext().execute_with(|| {
        let target = account_to_validator(1);

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(2),
            target,
            ViolationType::Minor
        ));

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(3),
            target,
            ViolationType::Moderate
        ));

        let disputes = Dispute::get_disputes_for_validator(target);
        assert_eq!(disputes.len(), 2);
        assert_eq!(disputes[0], DisputeId::new(0));
        assert_eq!(disputes[1], DisputeId::new(1));
    });
}

#[test]
fn open_disputes_tracking() {
    new_test_ext().execute_with(|| {
        let target1 = account_to_validator(1);
        let target2 = account_to_validator(2);

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(10),
            target1,
            ViolationType::Minor
        ));

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(10),
            target2,
            ViolationType::Severe
        ));

        assert_eq!(Dispute::get_open_dispute_count(), 2);

        assert_ok!(Dispute::resolve_dispute(
            RuntimeOrigin::root(),
            DisputeId::new(0),
            DisputeOutcome::ValidatorSlashed
        ));

        assert_eq!(Dispute::get_open_dispute_count(), 1);
    });
}

#[test]
fn is_dispute_open_helper() {
    new_test_ext().execute_with(|| {
        let target = account_to_validator(1);

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(2),
            target,
            ViolationType::Minor
        ));

        let dispute_id = DisputeId::new(0);

        assert!(Dispute::is_dispute_open(dispute_id));

        assert_ok!(Dispute::resolve_dispute(
            RuntimeOrigin::root(),
            dispute_id,
            DisputeOutcome::ValidatorSlashed
        ));

        assert!(!Dispute::is_dispute_open(dispute_id));
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        let target = account_to_validator(1);

        assert_ok!(Dispute::open_dispute(
            RuntimeOrigin::signed(2),
            target,
            ViolationType::Moderate
        ));

        System::assert_has_event(RuntimeEvent::Dispute(Event::DisputeOpened {
            dispute_id: DisputeId::new(0),
            reporter: 2,
            target,
            violation: ViolationType::Moderate,
        }));
    });
}
