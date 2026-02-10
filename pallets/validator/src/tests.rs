#![allow(clippy::disallowed_macros)]

use crate::{self as pallet_validator, Error, Event, ValidatorStatus};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
};
use frame_system as system;
use seveny_primitives::types::{ValidatorId, ViolationType};
use sp_arithmetic::Perbill;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, Hash, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Validator: pallet_validator,
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
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type Balance = u64;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ConstU64<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<0>;
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
    type DoneSlashHandler = ();
}

parameter_types! {
    pub const MinStake: u64 = 1000;
    pub const MaxValidators: u32 = 100;
    pub const BondingDuration: u64 = 10;
    pub const SlashDeferDuration: u64 = 5;
}

impl pallet_validator::Config for Test {
    type WeightInfo = ();
    type Currency = Balances;
    type MinStake = MinStake;
    type MaxValidators = MaxValidators;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 100_000),
            (2, 100_000),
            (3, 100_000),
            (4, 100_000),
            (5, 100_000),
            (6, 100_000),
            (7, 100_000),
            (8, 100_000),
            (9, 100_000),
            (10, 100_000),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .expect("balances genesis build failed");

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn new_test_ext_with_validators() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 100_000),
            (2, 100_000),
            (3, 100_000),
            (4, 100_000),
            (5, 100_000),
            (6, 100_000),
            (7, 100_000),
            (8, 100_000),
            (9, 100_000),
            (10, 100_000),
        ],
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .expect("balances genesis build failed");

    pallet_validator::GenesisConfig::<Test> {
        initial_validators: vec![
            (1, 10_000),
            (2, 10_000),
            (3, 10_000),
            (4, 10_000),
            (5, 10_000),
            (6, 10_000),
        ],
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("validator genesis build failed");

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn account_to_validator(account: u64) -> ValidatorId {
    let hash = BlakeTwo256::hash_of(&account);
    ValidatorId::from(H256(hash.0))
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
    }
}

#[test]
fn register_validator_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(1), 5000));

        let validator_id = account_to_validator(1);
        let info = Validator::validators(validator_id).expect("validator should exist");

        assert_eq!(info.stake, 5000);
        assert_eq!(info.status, ValidatorStatus::Bonding);
        assert_eq!(info.controller, 1);

        assert_eq!(Validator::validator_count(), 1);
        assert_eq!(Validator::total_stake(), 5000);
    });
}

#[test]
fn register_validator_insufficient_stake() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Validator::register_validator(RuntimeOrigin::signed(1), 500),
            Error::<Test>::InsufficientStake
        );
    });
}

#[test]
fn register_validator_duplicate() {
    new_test_ext().execute_with(|| {
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(1), 5000));

        assert_noop!(
            Validator::register_validator(RuntimeOrigin::signed(1), 5000),
            Error::<Test>::ControllerAlreadyUsed
        );
    });
}

#[test]
fn activate_validator_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(1), 5000));

        run_to_block(12);

        assert_ok!(Validator::activate_validator(RuntimeOrigin::signed(1)));

        let validator_id = account_to_validator(1);
        let info = Validator::validators(validator_id).expect("validator should exist");

        assert_eq!(info.status, ValidatorStatus::Active);
        assert_eq!(Validator::active_validator_count(), 1);
    });
}

#[test]
fn activate_validator_bonding_not_elapsed() {
    new_test_ext().execute_with(|| {
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(1), 5000));

        assert_noop!(
            Validator::activate_validator(RuntimeOrigin::signed(1)),
            Error::<Test>::BondingPeriodNotElapsed
        );
    });
}

#[test]
fn invariant_inv46_min_validators() {
    new_test_ext_with_validators().execute_with(|| {
        let active_count = Validator::active_validator_count();
        assert_eq!(active_count, 6);

        assert_ok!(Validator::deactivate_validator(RuntimeOrigin::signed(6)));
        assert_eq!(Validator::active_validator_count(), 5);

        assert_noop!(
            Validator::deactivate_validator(RuntimeOrigin::signed(5)),
            Error::<Test>::MinValidatorsRequired
        );

        assert_eq!(Validator::active_validator_count(), 5);
    });
}

#[test]
fn invariant_inv47_max_stake_ratio() {
    new_test_ext().execute_with(|| {
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(1), 3000));
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(2), 1400));

        assert_noop!(
            Validator::register_validator(RuntimeOrigin::signed(3), 10_000),
            Error::<Test>::StakeTooHigh
        );
    });
}

#[test]
fn invariant_inv48_slash_percentages() {
    new_test_ext_with_validators().execute_with(|| {
        let validator_id = account_to_validator(1);
        let initial_stake = Validator::validator_stake(validator_id);

        assert_ok!(Validator::slash_validator(
            RuntimeOrigin::root(),
            validator_id,
            ViolationType::Minor
        ));

        let slash = Perbill::from_percent(5).mul_floor(initial_stake);
        let pending = Validator::pending_slashes(0).expect("slash should exist");
        assert_eq!(pending.amount, slash);

        let validator_id2 = account_to_validator(2);
        assert_ok!(Validator::slash_validator(
            RuntimeOrigin::root(),
            validator_id2,
            ViolationType::Moderate
        ));

        let slash2 = Perbill::from_percent(20).mul_floor(initial_stake);
        let pending2 = Validator::pending_slashes(1).expect("slash should exist");
        assert_eq!(pending2.amount, slash2);

        let validator_id3 = account_to_validator(3);
        assert_ok!(Validator::slash_validator(
            RuntimeOrigin::root(),
            validator_id3,
            ViolationType::Severe
        ));

        let slash3 = Perbill::from_percent(50).mul_floor(initial_stake);
        let pending3 = Validator::pending_slashes(2).expect("slash should exist");
        assert_eq!(pending3.amount, slash3);

        let validator_id4 = account_to_validator(4);
        assert_ok!(Validator::slash_validator(
            RuntimeOrigin::root(),
            validator_id4,
            ViolationType::Critical
        ));

        let slash4 = Perbill::from_percent(100).mul_floor(initial_stake);
        let pending4 = Validator::pending_slashes(3).expect("slash should exist");
        assert_eq!(pending4.amount, slash4);
    });
}

#[test]
fn invariant_inv49_evidence_reward_capped() {
    new_test_ext_with_validators().execute_with(|| {
        let validator_id = account_to_validator(1);
        let initial_stake = Validator::validator_stake(validator_id);

        assert_ok!(Validator::report_evidence(
            RuntimeOrigin::signed(7),
            validator_id,
            ViolationType::Critical
        ));

        let slash_amount = Perbill::from_percent(100).mul_floor(initial_stake);
        let expected_reward = core::cmp::min(slash_amount / 10, 1000);

        System::assert_has_event(RuntimeEvent::Validator(Event::EvidenceRewardPaid {
            reporter: 7,
            amount: expected_reward,
        }));
    });
}

#[test]
fn deactivate_validator_success() {
    new_test_ext_with_validators().execute_with(|| {
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(7), 2000));
        run_to_block(12);
        assert_ok!(Validator::activate_validator(RuntimeOrigin::signed(7)));

        let initial_count = Validator::active_validator_count();

        assert_ok!(Validator::deactivate_validator(RuntimeOrigin::signed(7)));

        let validator_id = account_to_validator(7);
        let info = Validator::validators(validator_id).expect("validator should exist");

        assert_eq!(info.status, ValidatorStatus::Unbonding);
        assert_eq!(Validator::active_validator_count(), initial_count - 1);
    });
}

#[test]
fn withdraw_stake_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(1), 3000));
        run_to_block(12);
        assert_ok!(Validator::activate_validator(RuntimeOrigin::signed(1)));

        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(2), 1200));
        run_to_block(24);
        assert_ok!(Validator::activate_validator(RuntimeOrigin::signed(2)));

        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(3), 1200));
        run_to_block(36);
        assert_ok!(Validator::activate_validator(RuntimeOrigin::signed(3)));

        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(4), 1200));
        run_to_block(48);
        assert_ok!(Validator::activate_validator(RuntimeOrigin::signed(4)));

        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(5), 1200));
        run_to_block(60);
        assert_ok!(Validator::activate_validator(RuntimeOrigin::signed(5)));

        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(6), 1200));
        run_to_block(72);
        assert_ok!(Validator::activate_validator(RuntimeOrigin::signed(6)));

        assert_ok!(Validator::deactivate_validator(RuntimeOrigin::signed(6)));

        run_to_block(83);

        assert_ok!(Validator::withdraw_stake(RuntimeOrigin::signed(6)));

        let validator_id = account_to_validator(6);
        assert!(Validator::validators(validator_id).is_none());
    });
}

#[test]
fn withdraw_stake_unbonding_not_elapsed() {
    new_test_ext().execute_with(|| {
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(1), 3000));
        run_to_block(12);
        assert_ok!(Validator::activate_validator(RuntimeOrigin::signed(1)));

        for i in 2..=6 {
            assert_ok!(Validator::register_validator(
                RuntimeOrigin::signed(i),
                1200
            ));
            run_to_block(12 + (i - 1) * 12);
            assert_ok!(Validator::activate_validator(RuntimeOrigin::signed(i)));
        }

        assert_ok!(Validator::deactivate_validator(RuntimeOrigin::signed(6)));

        assert_noop!(
            Validator::withdraw_stake(RuntimeOrigin::signed(6)),
            Error::<Test>::UnbondingPeriodNotElapsed
        );
    });
}

#[test]
fn increase_stake_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(1), 3000));
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(2), 1200));

        assert_ok!(Validator::increase_stake(RuntimeOrigin::signed(2), 500));

        let validator_id = account_to_validator(2);
        let info = Validator::validators(validator_id).expect("validator should exist");

        assert_eq!(info.stake, 1700);
        assert_eq!(Validator::validator_stake(validator_id), 1700);
        assert_eq!(Validator::total_stake(), 4700);
    });
}

#[test]
fn slash_validator_defers_slash() {
    new_test_ext_with_validators().execute_with(|| {
        let validator_id = account_to_validator(1);

        assert_ok!(Validator::slash_validator(
            RuntimeOrigin::root(),
            validator_id,
            ViolationType::Minor
        ));

        let pending = Validator::pending_slashes(0).expect("slash should exist");
        assert!(!pending.applied);
        assert_eq!(pending.validator, validator_id);
    });
}

#[test]
fn apply_slash_success() {
    new_test_ext_with_validators().execute_with(|| {
        let validator_id = account_to_validator(1);
        let initial_stake = Validator::validator_stake(validator_id);

        assert_ok!(Validator::slash_validator(
            RuntimeOrigin::root(),
            validator_id,
            ViolationType::Minor
        ));

        run_to_block(7);

        assert_ok!(Validator::apply_slash(RuntimeOrigin::root(), 0));

        let pending = Validator::pending_slashes(0).expect("slash should exist");
        assert!(pending.applied);

        let slash_amount = Perbill::from_percent(5).mul_floor(initial_stake);
        let new_stake = Validator::validator_stake(validator_id);
        assert_eq!(new_stake, initial_stake - slash_amount);
    });
}

#[test]
fn apply_slash_defer_not_elapsed() {
    new_test_ext_with_validators().execute_with(|| {
        let validator_id = account_to_validator(1);

        assert_ok!(Validator::slash_validator(
            RuntimeOrigin::root(),
            validator_id,
            ViolationType::Minor
        ));

        assert_noop!(
            Validator::apply_slash(RuntimeOrigin::root(), 0),
            Error::<Test>::UnbondingPeriodNotElapsed
        );
    });
}

#[test]
fn double_voting_immediately_slashes_status() {
    new_test_ext_with_validators().execute_with(|| {
        let validator_id = account_to_validator(1);
        let initial_active = Validator::active_validator_count();

        assert_ok!(Validator::slash_validator(
            RuntimeOrigin::root(),
            validator_id,
            ViolationType::Critical
        ));

        let info = Validator::validators(validator_id).expect("validator should exist");
        assert_eq!(info.status, ValidatorStatus::Slashed);
        assert_eq!(Validator::active_validator_count(), initial_active - 1);
    });
}

#[test]
fn get_active_validators_helper() {
    new_test_ext_with_validators().execute_with(|| {
        let active = Validator::get_active_validators();
        assert_eq!(active.len(), 6);
    });
}

#[test]
fn get_stake_ratio_helper() {
    new_test_ext_with_validators().execute_with(|| {
        let validator_id = account_to_validator(1);
        let (stake, total) = Validator::get_stake_ratio(validator_id).expect("ratio should exist");

        assert_eq!(stake, 10_000);
        assert_eq!(total, 60_000);
    });
}

#[test]
fn is_validator_active_helper() {
    new_test_ext_with_validators().execute_with(|| {
        let validator_id = account_to_validator(1);
        assert!(Validator::is_validator_active(validator_id));

        let non_existent = ValidatorId::from_raw([99u8; 32]);
        assert!(!Validator::is_validator_active(non_existent));
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        assert_ok!(Validator::register_validator(RuntimeOrigin::signed(1), 5000));

        let validator_id = account_to_validator(1);

        System::assert_has_event(RuntimeEvent::Validator(Event::ValidatorRegistered {
            validator: validator_id,
            controller: 1,
            stake: 5000,
        }));
    });
}
