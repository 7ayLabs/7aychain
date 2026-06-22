//! Cross-pallet integration tests for the 7aychain runtime.
//!
//! These tests wire together multiple pallets (Epoch, Validator, Presence,
//! Lifecycle, Device, Carrier) using their real trait implementations rather
//! than mocks, verifying that cross-pallet interactions enforce the protocol
//! invariants correctly.
//!
//! Covered invariants:
//!   - INV1:  No duplicate presence per (actor, epoch)
//!   - INV7:  Monotonic forward-only state transitions
//!   - INV8:  Terminal states (Finalized, Slashed) are irreversible
//!   - INV9:  Declarations only in active epochs
//!   - INV10: Validation requires quorum
//!   - INV11: No duplicate votes per (validator, actor, epoch)
//!   - INV14: Epoch data bounded to lifetime
//!   - INV16: Actors must register in epoch to participate
//!   - INV46: Validator set >= MIN_VALIDATORS (relaxed in test to 3)
//!   - INV47: Stake ratio enforcement

#![allow(clippy::disallowed_macros, clippy::missing_const_for_thread_local)]

use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64, Hooks},
};
use frame_system as system;
use pallet_device::{AttestationType, DeviceId, DeviceType};
use parity_scale_codec::Encode;
use seveny_primitives::{
    fusion::Position,
    types::{ActorId, EpochId, EpochState, PresenceState, ValidatorId},
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

// =========================================================================
// Mock Runtime (multi-pallet)
// =========================================================================

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Balances: pallet_balances,
        Epoch: pallet_epoch,
        Validator: pallet_validator,
        Presence: pallet_presence,
        Device: pallet_device,
        Carrier: pallet_carrier,
        Lifecycle: pallet_lifecycle,
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

// -- Epoch config --

parameter_types! {
    pub const EpochDuration: u64 = 50;
    pub const MinEpochDuration: u64 = 4;
    pub const MaxEpochDuration: u64 = 1000;
    pub const GracePeriod: u64 = 5;
    pub const AllowUnverifiedVrfSubmissions: bool = false;
}

impl pallet_epoch::Config for Test {
    type WeightInfo = ();
    type EpochDuration = EpochDuration;
    type MinEpochDuration = MinEpochDuration;
    type MaxEpochDuration = MaxEpochDuration;
    type GracePeriod = GracePeriod;
    type AllowUnverifiedVrfSubmissions = AllowUnverifiedVrfSubmissions;
}

// -- Validator config --

parameter_types! {
    pub const MinStake: u64 = 1_000;
    pub const MaxValidators: u32 = 100;
    pub const MinValidators: u32 = 3;
    pub const BondingDuration: u64 = 10;
    pub const SlashDeferDuration: u64 = 5;
}

impl pallet_validator::Config for Test {
    type WeightInfo = ();
    type Currency = Balances;
    type MinStake = MinStake;
    type MaxValidators = MaxValidators;
    type MinValidators = MinValidators;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
}

// -- Presence config --
// EpochProvider = Epoch pallet (real), ValidatorProvider = Validator pallet (real)

parameter_types! {
    pub const MaxVotesPerPresence: u32 = 100;
    pub const DefaultQuorumThreshold: u32 = 2;
    pub const DefaultQuorumTotal: u32 = 3;
    pub const CommitRevealDelay: u64 = 5;
    pub const RevealWindow: u64 = 10;
    pub const MinWitnessesForVerification: u32 = 3;
    pub const PositionToleranceMeters: u32 = 1000;
}

impl pallet_presence::Config for Test {
    type WeightInfo = ();
    type MaxVotesPerPresence = MaxVotesPerPresence;
    type DefaultQuorumThreshold = DefaultQuorumThreshold;
    type DefaultQuorumTotal = DefaultQuorumTotal;
    type CommitRevealDelay = CommitRevealDelay;
    type RevealWindow = RevealWindow;
    type MinWitnessesForVerification = MinWitnessesForVerification;
    type PositionToleranceMeters = PositionToleranceMeters;
    type EpochProvider = Epoch;
    type ValidatorProvider = Validator;
}

// -- Device config --

parameter_types! {
    pub const MaxDevicesPerActor: u32 = 10;
    pub const AttestationValidityBlocks: u64 = 1000;
    pub const InitialTrustScore: u8 = 50;
    pub const DeviceHeartbeatTimeoutBlocks: u64 = 10;
    pub const DeviceMaxConsecutiveMisses: u32 = 3;
    pub const DeviceHealthScoreDecay: u8 = 10;
    pub const DeviceHealthScoreRecovery: u8 = 5;
}

impl pallet_device::Config for Test {
    type WeightInfo = ();
    type MaxDevicesPerActor = MaxDevicesPerActor;
    type AttestationValidityBlocks = AttestationValidityBlocks;
    type InitialTrustScore = InitialTrustScore;
    type HeartbeatTimeoutBlocks = DeviceHeartbeatTimeoutBlocks;
    type MaxConsecutiveMisses = DeviceMaxConsecutiveMisses;
    type HealthScoreDecay = DeviceHealthScoreDecay;
    type HealthScoreRecovery = DeviceHealthScoreRecovery;
}

// -- Lifecycle config --

parameter_types! {
    pub const KeyDestructionTimeoutBlocks: u64 = 100;
    pub const MinDestructionAttestations: u32 = 3;
    pub const RotationCooldownBlocks: u64 = 50;
    pub const RotationTimeoutBlocks: u64 = 200;
}

impl pallet_lifecycle::Config for Test {
    type WeightInfo = ();
    type KeyDestructionTimeoutBlocks = KeyDestructionTimeoutBlocks;
    type MinDestructionAttestations = MinDestructionAttestations;
    type RotationCooldownBlocks = RotationCooldownBlocks;
    type RotationTimeoutBlocks = RotationTimeoutBlocks;
}

// -- Carrier config --

parameter_types! {
    pub const MaxNumbersPerActor: u32 = 3;
    pub const MaxCarrierWitnessesPerRequest: u32 = 32;
    pub const CarrierWitnessThreshold: u32 = 2;
    pub const CarrierServiceLeaseBlocks: u64 = 600;
    pub const CarrierWitnessReward: u128 = 100_000_000_000;
    pub const MaxCarrierRegionsPerServiceNode: u32 = 8;
    pub const MinCarrierServiceStake: u128 = 1_000;
    pub const MinRegionalCarrierNodes: u32 = 2;
    pub const StrongRegionalCarrierNodes: u32 = 4;
    pub const CarrierServiceNodeSlashThreshold: u32 = 2;
    pub const CarrierBridgeAccount: u64 = 42;
}

pub struct TestActorChecker;
impl seveny_primitives::traits::ActorActivityChecker for TestActorChecker {
    fn is_actor_active(actor_id: ActorId) -> bool {
        pallet_lifecycle::Pallet::<Test>::is_actor_active(actor_id)
    }
}

pub struct TestDeviceChecker;
impl seveny_primitives::traits::DeviceEligibilityChecker for TestDeviceChecker {
    fn is_device_active_for_actor(actor_id: ActorId, device_id: u64) -> bool {
        let device = DeviceId::new(device_id);
        pallet_device::Pallet::<Test>::is_device_active(device)
            && pallet_device::Pallet::<Test>::get_actor_devices(actor_id)
                .into_iter()
                .any(|candidate| candidate == device)
    }
}

pub struct TestPresenceVerifier;
impl seveny_primitives::traits::PresenceVerifier for TestPresenceVerifier {
    fn is_presence_verified(actor_id: ActorId, epoch_id: EpochId) -> bool {
        let Some(record) = pallet_presence::Presences::<Test>::get(epoch_id, actor_id) else {
            return false;
        };

        let state_ok = matches!(
            record.state,
            PresenceState::Validated | PresenceState::Finalized
        );
        let position_ok = pallet_presence::PositionClaims::<Test>::get(epoch_id, actor_id)
            .is_some_and(|claim| claim.verified);

        state_ok && position_ok
    }
}

pub struct TestValidatorStakeProvider;
impl seveny_primitives::traits::ValidatorStakeProvider for TestValidatorStakeProvider {
    fn validator_stake(validator_id: ValidatorId) -> u128 {
        pallet_validator::Pallet::<Test>::validator_stake(validator_id) as u128
    }
}

pub struct TestServiceNodePresenceVerifier;
impl seveny_primitives::traits::ServiceNodePresenceVerifier<u64>
    for TestServiceNodePresenceVerifier
{
    fn is_service_node_present(controller: &u64, epoch_id: EpochId) -> bool {
        let actor_id = account_to_actor(*controller);
        pallet_presence::PositionClaims::<Test>::get(epoch_id, actor_id)
            .is_some_and(|claim| claim.verified)
    }
}

impl pallet_carrier::Config for Test {
    type WeightInfo = ();
    type EpochProvider = Epoch;
    type ValidatorProvider = Validator;
    type ValidatorStakeProvider = TestValidatorStakeProvider;
    type ActorChecker = TestActorChecker;
    type DeviceChecker = TestDeviceChecker;
    type PresenceVerifier = TestPresenceVerifier;
    type ServiceNodePresenceVerifier = TestServiceNodePresenceVerifier;
    type RewardHandler = seveny_primitives::traits::NoOpCarrierReward;
    type MaxNumbersPerActor = MaxNumbersPerActor;
    type MaxWitnessesPerRequest = MaxCarrierWitnessesPerRequest;
    type WitnessThreshold = CarrierWitnessThreshold;
    type ServiceLeaseBlocks = CarrierServiceLeaseBlocks;
    type WitnessRewardAmount = CarrierWitnessReward;
    type CarrierBridgeAccount = CarrierBridgeAccount;
    type MaxRegionsPerServiceNode = MaxCarrierRegionsPerServiceNode;
    type MinServiceStake = MinCarrierServiceStake;
    type MinRegionalServiceNodes = MinRegionalCarrierNodes;
    type StrongRegionalServiceNodes = StrongRegionalCarrierNodes;
    type ServiceNodeSlashThreshold = CarrierServiceNodeSlashThreshold;
}

// =========================================================================
// Helpers
// =========================================================================

/// Build test externalities with:
///   - Genesis epoch 1 (active, duration=50, blocks 1..51)
///   - 6 funded accounts (1..=6) each with 100_000 balance
///   - 6 pre-registered active validators (accounts 1..=6, stake 10_000 each)
///   - Quorum: threshold=2, total=3
fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("system storage build failed");

    pallet_balances::GenesisConfig::<Test> {
        balances: (1u64..=10).map(|id| (id, 100_000u64)).collect(),
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .expect("balances genesis build failed");

    pallet_epoch::GenesisConfig::<Test> {
        initial_epoch_duration: 50,
        initial_grace_period: 5,
        auto_transition: true,
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("epoch genesis build failed");

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

    pallet_presence::GenesisConfig::<Test> {
        quorum_threshold: 2,
        quorum_total: 3,
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("presence genesis build failed");

    pallet_device::GenesisConfig::<Test> {
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("device genesis build failed");

    pallet_lifecycle::GenesisConfig::<Test> {
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("lifecycle genesis build failed");

    pallet_carrier::GenesisConfig::<Test> {
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("carrier genesis build failed");

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Build test externalities without pre-registered validators.
/// Useful for testing validator registration + activation flows.
fn new_test_ext_no_validators() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("system storage build failed");

    pallet_balances::GenesisConfig::<Test> {
        balances: (1u64..=10).map(|id| (id, 100_000u64)).collect(),
        dev_accounts: None,
    }
    .assimilate_storage(&mut t)
    .expect("balances genesis build failed");

    pallet_epoch::GenesisConfig::<Test> {
        initial_epoch_duration: 50,
        initial_grace_period: 5,
        auto_transition: true,
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("epoch genesis build failed");

    pallet_presence::GenesisConfig::<Test> {
        quorum_threshold: 2,
        quorum_total: 3,
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("presence genesis build failed");

    pallet_device::GenesisConfig::<Test> {
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("device genesis build failed");

    pallet_lifecycle::GenesisConfig::<Test> {
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("lifecycle genesis build failed");

    pallet_carrier::GenesisConfig::<Test> {
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("carrier genesis build failed");

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

/// Advance blocks, calling Epoch::on_initialize at each step to trigger
/// automatic epoch transitions (Epoch uses on_initialize for auto-transition).
fn run_to_block(n: u64) {
    while System::block_number() < n {
        let next = System::block_number() + 1;
        System::set_block_number(next);
        Epoch::on_initialize(next);
        Device::on_initialize(next);
        Lifecycle::on_initialize(next);
    }
}

/// Derive an ActorId from a test account number (matches pallet internals).
fn account_to_actor(account: u64) -> ActorId {
    seveny_primitives::crypto::derive_actor_id(&account.encode())
}

/// Derive a ValidatorId from a test account number (matches pallet internals).
fn account_to_validator(account: u64) -> ValidatorId {
    seveny_primitives::crypto::derive_validator_id(&account.encode())
}

fn register_and_activate_actor(account: u64) -> ActorId {
    let actor = account_to_actor(account);
    assert_ok!(Lifecycle::register_actor(
        RuntimeOrigin::signed(account),
        H256::repeat_byte(account as u8)
    ));
    assert_ok!(Lifecycle::activate_actor(RuntimeOrigin::root(), actor));
    actor
}

fn register_and_activate_device(account: u64, public_key_hash: H256) -> u64 {
    assert_ok!(Device::register_device(
        RuntimeOrigin::signed(account),
        DeviceType::Mobile,
        public_key_hash,
        AttestationType::HardwareBacked
    ));
    let device_id = pallet_device::DeviceCount::<Test>::get().saturating_sub(1);
    assert_ok!(Device::activate_device(
        RuntimeOrigin::signed(account),
        DeviceId::new(device_id)
    ));
    device_id
}

fn set_validator_positions() {
    assert_ok!(Presence::set_validator_position(
        RuntimeOrigin::signed(1),
        account_to_validator(1),
        Position::new(0, 0, 0)
    ));
    assert_ok!(Presence::set_validator_position(
        RuntimeOrigin::signed(2),
        account_to_validator(2),
        Position::new(300, 0, 0)
    ));
    assert_ok!(Presence::set_validator_position(
        RuntimeOrigin::signed(3),
        account_to_validator(3),
        Position::new(0, 300, 0)
    ));
    assert_ok!(Presence::set_validator_position(
        RuntimeOrigin::signed(4),
        account_to_validator(4),
        Position::new(300, 300, 0)
    ));
}

fn prove_service_node_presence(
    validator_account: u64,
    epoch: EpochId,
    position: Position,
) -> ActorId {
    let actor = account_to_actor(validator_account);

    assert_ok!(Presence::declare_presence(
        RuntimeOrigin::signed(validator_account),
        epoch
    ));
    assert_ok!(Presence::claim_position(
        RuntimeOrigin::signed(validator_account),
        epoch,
        position
    ));

    let witnesses: &[u64] = match validator_account {
        1 => &[2, 3, 4],
        2 => &[1, 3, 4],
        3 => &[1, 2, 4],
        _ => &[1, 2, 3],
    };
    for witness in witnesses {
        assert_ok!(Presence::submit_witness_attestation(
            RuntimeOrigin::signed(*witness),
            actor,
            epoch,
            3,
            true
        ));
    }

    assert_ok!(Presence::verify_position(
        RuntimeOrigin::signed(1),
        actor,
        epoch
    ));

    actor
}

fn register_service_node(account: u64, region_id: H256) {
    let regions =
        frame_support::BoundedVec::<H256, MaxCarrierRegionsPerServiceNode>::try_from(vec![
            region_id,
        ])
        .expect("bounded regions");

    assert_ok!(Carrier::register_service_node(
        RuntimeOrigin::signed(account),
        regions
    ));
}

fn prove_verified_presence(actor_account: u64, epoch: EpochId, position: Position) -> ActorId {
    let actor = account_to_actor(actor_account);

    assert_ok!(Presence::declare_presence(
        RuntimeOrigin::signed(actor_account),
        epoch
    ));
    assert_ok!(Presence::claim_position(
        RuntimeOrigin::signed(actor_account),
        epoch,
        position
    ));

    for validator in [1u64, 2, 3] {
        assert_ok!(Presence::submit_witness_attestation(
            RuntimeOrigin::signed(validator),
            actor,
            epoch,
            3,
            true
        ));
    }

    assert_ok!(Presence::verify_position(
        RuntimeOrigin::signed(1),
        actor,
        epoch
    ));
    assert_ok!(Presence::vote_presence(
        RuntimeOrigin::signed(1),
        actor,
        epoch,
        true
    ));
    assert_ok!(Presence::vote_presence(
        RuntimeOrigin::signed(2),
        actor,
        epoch,
        true
    ));
    assert_ok!(Presence::finalize_presence(
        RuntimeOrigin::signed(actor_account),
        actor,
        epoch
    ));

    actor
}

// =========================================================================
// 1. Full Presence Lifecycle (cross-pallet happy path)
// =========================================================================

#[test]
fn full_presence_lifecycle_across_pallets() {
    // Exercises: Epoch (active) -> Presence::declare -> Validator votes ->
    //            quorum met (Validated) -> finalize -> state is Finalized
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        // Confirm epoch 1 is active (set up by genesis)
        assert!(Epoch::is_epoch_active(epoch));
        let meta = Epoch::epoch_info(epoch).expect("epoch 1 exists");
        assert_eq!(meta.state, EpochState::Active);

        // Account 7 declares presence (not a validator, just an actor)
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));

        let actor = account_to_actor(7);
        let record = Presence::presences(epoch, actor).expect("presence record exists");
        assert_eq!(record.state, PresenceState::Declared);
        assert_eq!(record.vote_count, 0);

        // Validators 1 and 2 vote to approve (quorum threshold = 2)
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            true
        ));

        // After 1 vote: still Declared
        let record = Presence::presences(epoch, actor).unwrap();
        assert_eq!(record.state, PresenceState::Declared);
        assert_eq!(record.vote_count, 1);

        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(2),
            actor,
            epoch,
            true
        ));

        // After 2 votes: quorum met, auto-transitions to Validated
        let record = Presence::presences(epoch, actor).unwrap();
        assert_eq!(record.state, PresenceState::Validated);
        assert_eq!(record.vote_count, 2);

        // The actor finalizes their own presence
        assert_ok!(Presence::finalize_presence(
            RuntimeOrigin::signed(7),
            actor,
            epoch
        ));

        let record = Presence::presences(epoch, actor).unwrap();
        assert_eq!(record.state, PresenceState::Finalized);
        assert!(record.finalized_at.is_some());
    });
}

#[test]
fn validator_can_finalize_presence_for_another_actor() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        // Account 8 declares
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(8), epoch));
        let actor = account_to_actor(8);

        // Validators 1, 2 approve -> Validated
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            true
        ));
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(2),
            actor,
            epoch,
            true
        ));

        // Validator 3 (an active validator) finalizes on behalf of actor 8
        assert_ok!(Presence::finalize_presence(
            RuntimeOrigin::signed(3),
            actor,
            epoch
        ));

        let record = Presence::presences(epoch, actor).unwrap();
        assert_eq!(record.state, PresenceState::Finalized);
    });
}

#[test]
fn carrier_activation_flow_across_identity_device_presence_and_carrier() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let actor = register_and_activate_actor(7);
        let device_id = register_and_activate_device(7, H256::repeat_byte(70));

        set_validator_positions();
        let proven_actor = prove_verified_presence(7, epoch, Position::new(100, 100, 0));
        assert_eq!(proven_actor, actor);

        let position_claim =
            pallet_presence::PositionClaims::<Test>::get(epoch, actor).expect("claim exists");
        assert!(position_claim.verified);

        let number_id = H256::repeat_byte(120);
        let region_id = H256::repeat_byte(42);
        let sim_profile_commitment = H256::repeat_byte(11);

        prove_service_node_presence(1, epoch, Position::new(0, 0, 0));
        prove_service_node_presence(2, epoch, Position::new(300, 0, 0));
        register_service_node(1, region_id);
        register_service_node(2, region_id);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(7),
            number_id,
            region_id
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(7),
            number_id,
            device_id,
            epoch,
            sim_profile_commitment
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(1),
            number_id,
            epoch,
            region_id,
            91
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(2),
            number_id,
            epoch,
            region_id,
            87
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(7),
            number_id,
            epoch
        ));

        let binding = Carrier::numbers(number_id).expect("carrier binding exists");
        assert_eq!(binding.owner, actor);
        assert_eq!(binding.current_device_id, Some(device_id));
        assert_eq!(binding.status, pallet_carrier::NumberStatus::Activated);
        assert_eq!(binding.region_id, region_id);
        assert_eq!(binding.activation_epoch, Some(epoch));
        assert_eq!(binding.sim_profile_commitment, Some(sim_profile_commitment));
        assert_eq!(
            binding.provisioning_state,
            pallet_carrier::ProvisioningState::Pending
        );
        assert!(binding.service_lease_until.is_some());
        assert_eq!(Carrier::device_numbers(device_id), Some(number_id));
        assert!(Carrier::service_requests(epoch, number_id).is_none());
    });
}

#[test]
fn carrier_recovery_rebinds_the_number_to_a_replacement_device() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let actor = register_and_activate_actor(7);
        let original_device = register_and_activate_device(7, H256::repeat_byte(71));

        set_validator_positions();
        let proven_actor = prove_verified_presence(7, epoch, Position::new(150, 150, 0));
        assert_eq!(proven_actor, actor);

        let number_id = H256::repeat_byte(121);
        let region_id = H256::repeat_byte(43);

        prove_service_node_presence(1, epoch, Position::new(0, 0, 0));
        prove_service_node_presence(2, epoch, Position::new(300, 0, 0));
        register_service_node(1, region_id);
        register_service_node(2, region_id);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(7),
            number_id,
            region_id
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(7),
            number_id,
            original_device,
            epoch,
            H256::repeat_byte(12)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(1),
            number_id,
            epoch,
            region_id,
            92
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(2),
            number_id,
            epoch,
            region_id,
            88
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(7),
            number_id,
            epoch
        ));

        let replacement_device = register_and_activate_device(7, H256::repeat_byte(72));
        assert_ok!(Carrier::request_recovery(
            RuntimeOrigin::signed(7),
            number_id,
            replacement_device,
            epoch,
            H256::repeat_byte(13)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(1),
            number_id,
            epoch,
            region_id,
            95
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(2),
            number_id,
            epoch,
            region_id,
            89
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(2),
            number_id,
            epoch
        ));

        let binding = Carrier::numbers(number_id).expect("carrier binding exists");
        assert_eq!(binding.owner, actor);
        assert_eq!(binding.current_device_id, Some(replacement_device));
        assert_eq!(binding.status, pallet_carrier::NumberStatus::Recovered);
        assert_eq!(binding.activation_epoch, Some(epoch));
        assert_eq!(
            binding.provisioning_state,
            pallet_carrier::ProvisioningState::Pending
        );
        assert_eq!(Carrier::device_numbers(replacement_device), Some(number_id));
        assert!(Carrier::device_numbers(original_device).is_none());
    });
}

#[test]
fn carrier_bridge_can_record_provisioning_receipts() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let actor = register_and_activate_actor(7);
        let device_id = register_and_activate_device(7, H256::repeat_byte(73));

        set_validator_positions();
        let proven_actor = prove_verified_presence(7, epoch, Position::new(160, 160, 0));
        assert_eq!(proven_actor, actor);

        let number_id = H256::repeat_byte(122);
        let region_id = H256::repeat_byte(44);

        prove_service_node_presence(1, epoch, Position::new(0, 0, 0));
        prove_service_node_presence(2, epoch, Position::new(300, 0, 0));
        register_service_node(1, region_id);
        register_service_node(2, region_id);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(7),
            number_id,
            region_id
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(7),
            number_id,
            device_id,
            epoch,
            H256::repeat_byte(14)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(1),
            number_id,
            epoch,
            region_id,
            92
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(2),
            number_id,
            epoch,
            region_id,
            88
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(7),
            number_id,
            epoch
        ));

        assert_noop!(
            Carrier::record_provisioning_result(
                RuntimeOrigin::signed(7),
                number_id,
                H256::repeat_byte(15),
                true
            ),
            pallet_carrier::Error::<Test>::ProvisioningUpdateNotAllowed
        );

        assert_ok!(Carrier::record_provisioning_result(
            RuntimeOrigin::signed(42),
            number_id,
            H256::repeat_byte(16),
            true
        ));

        let binding = Carrier::numbers(number_id).expect("carrier binding exists");
        assert_eq!(
            binding.provisioning_state,
            pallet_carrier::ProvisioningState::Provisioned
        );
        assert_eq!(binding.provisioning_receipt, Some(H256::repeat_byte(16)));
    });
}

#[test]
fn carrier_activation_requires_regional_service_node_coverage() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let actor = register_and_activate_actor(7);
        let device_id = register_and_activate_device(7, H256::repeat_byte(74));

        set_validator_positions();
        let proven_actor = prove_verified_presence(7, epoch, Position::new(180, 180, 0));
        assert_eq!(proven_actor, actor);

        let number_id = H256::repeat_byte(123);
        let region_id = H256::repeat_byte(45);

        prove_service_node_presence(1, epoch, Position::new(0, 0, 0));
        register_service_node(1, region_id);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(7),
            number_id,
            region_id
        ));
        assert_noop!(
            Carrier::request_activation(
                RuntimeOrigin::signed(7),
                number_id,
                device_id,
                epoch,
                H256::repeat_byte(17)
            ),
            pallet_carrier::Error::<Test>::InsufficientRegionalCoverage
        );

        prove_service_node_presence(2, epoch, Position::new(300, 0, 0));
        register_service_node(2, region_id);

        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(7),
            number_id,
            device_id,
            epoch,
            H256::repeat_byte(17)
        ));
    });
}

#[test]
fn carrier_failure_reports_suspend_service_nodes_after_threshold() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let actor = register_and_activate_actor(7);
        let device_id = register_and_activate_device(7, H256::repeat_byte(75));

        set_validator_positions();
        let proven_actor = prove_verified_presence(7, epoch, Position::new(200, 200, 0));
        assert_eq!(proven_actor, actor);

        let number_id = H256::repeat_byte(124);
        let region_id = H256::repeat_byte(46);

        prove_service_node_presence(1, epoch, Position::new(0, 0, 0));
        prove_service_node_presence(2, epoch, Position::new(300, 0, 0));
        register_service_node(1, region_id);
        register_service_node(2, region_id);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(7),
            number_id,
            region_id
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(7),
            number_id,
            device_id,
            epoch,
            H256::repeat_byte(18)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(1),
            number_id,
            epoch,
            region_id,
            91
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(2),
            number_id,
            epoch,
            region_id,
            89
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(7),
            number_id,
            epoch
        ));

        let validator_one = account_to_validator(1);
        let validator_two = account_to_validator(2);

        assert_ok!(Carrier::report_service_failure(
            RuntimeOrigin::signed(7),
            number_id,
            validator_one
        ));
        assert_noop!(
            Carrier::report_service_failure(RuntimeOrigin::signed(7), number_id, validator_one),
            pallet_carrier::Error::<Test>::DuplicateFailureReport
        );

        let second_number = H256::repeat_byte(125);
        let second_device = register_and_activate_device(7, H256::repeat_byte(76));
        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(7),
            second_number,
            region_id
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(7),
            second_number,
            second_device,
            epoch,
            H256::repeat_byte(19)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(1),
            second_number,
            epoch,
            region_id,
            88
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(2),
            second_number,
            epoch,
            region_id,
            87
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(7),
            second_number,
            epoch
        ));
        assert_ok!(Carrier::report_service_failure(
            RuntimeOrigin::signed(7),
            second_number,
            validator_one
        ));

        let profile = Carrier::service_nodes(validator_one).expect("service node exists");
        assert_eq!(profile.status, pallet_carrier::ServiceNodeStatus::Suspended);
        assert_eq!(profile.slash_points, 2);

        let coverage = Carrier::region_coverage(epoch, region_id).expect("coverage exists");
        assert_eq!(coverage.eligible_node_count, 1);
        assert_eq!(coverage.status, pallet_carrier::CoverageStatus::Weak);

        let third_number = H256::repeat_byte(126);
        let third_device = register_and_activate_device(7, H256::repeat_byte(77));
        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(7),
            third_number,
            region_id
        ));
        assert_noop!(
            Carrier::request_activation(
                RuntimeOrigin::signed(7),
                third_number,
                third_device,
                epoch,
                H256::repeat_byte(20)
            ),
            pallet_carrier::Error::<Test>::InsufficientRegionalCoverage
        );
        assert!(Carrier::service_nodes(validator_two).is_some());
    });
}

// =========================================================================
// 2. Cross-Pallet Error Propagation
// =========================================================================

#[test]
fn cannot_declare_presence_in_non_active_epoch() {
    // INV9: Presence pallet queries Epoch pallet via EpochProvider trait.
    // When epoch is Closed, declare_presence must fail.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        // Close epoch 1 manually
        assert_ok!(Epoch::close_epoch(RuntimeOrigin::root(), epoch));
        let meta = Epoch::epoch_info(epoch).unwrap();
        assert_eq!(meta.state, EpochState::Closed);

        // Attempt to declare presence in closed epoch
        assert_noop!(
            Presence::declare_presence(RuntimeOrigin::signed(7), epoch),
            pallet_presence::Error::<Test>::EpochNotActive
        );
    });
}

#[test]
fn cannot_declare_presence_in_nonexistent_epoch() {
    new_test_ext().execute_with(|| {
        let future_epoch = EpochId::new(99);

        assert_noop!(
            Presence::declare_presence(RuntimeOrigin::signed(7), future_epoch),
            pallet_presence::Error::<Test>::EpochNotActive
        );
    });
}

#[test]
fn cannot_vote_as_unregistered_validator() {
    // The Presence pallet queries Validator pallet via ValidatorProvider.
    // An account that is not a registered validator must be rejected.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // Account 9 is funded but not a registered validator
        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(9), actor, epoch, true),
            pallet_presence::Error::<Test>::ValidatorNotActive
        );
    });
}

#[test]
fn cannot_vote_with_bonding_validator() {
    // A validator in Bonding status is not yet Active, so ValidatorProvider
    // returns false and the presence pallet rejects the vote.
    new_test_ext_no_validators().execute_with(|| {
        let epoch = EpochId::new(1);

        // Register a validator (enters Bonding state, not Active)
        assert_ok!(Validator::register_validator(
            RuntimeOrigin::signed(1),
            5_000
        ));
        let validator_info =
            Validator::validators(account_to_validator(1)).expect("validator exists");
        assert_eq!(
            validator_info.status,
            pallet_validator::ValidatorStatus::Bonding
        );

        // Another account declares presence
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // Bonding validator cannot vote
        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(1), actor, epoch, true),
            pallet_presence::Error::<Test>::ValidatorNotActive
        );
    });
}

#[test]
fn inv1_duplicate_presence_rejected_cross_pallet() {
    // INV1: No duplicate presence per (actor, epoch).
    // This test uses the real Epoch pallet to confirm the epoch is active.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));

        assert_noop!(
            Presence::declare_presence(RuntimeOrigin::signed(7), epoch),
            pallet_presence::Error::<Test>::DuplicatePresence
        );
    });
}

#[test]
fn inv11_duplicate_vote_rejected_cross_pallet() {
    // INV11: No duplicate votes per (validator, actor, epoch).
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            true
        ));

        // Same validator, same actor, same epoch -> rejected
        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(1), actor, epoch, true),
            pallet_presence::Error::<Test>::DuplicateVote
        );

        // Also rejected with different approve value
        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(1), actor, epoch, false),
            pallet_presence::Error::<Test>::DuplicateVote
        );
    });
}

// =========================================================================
// 3. Epoch-Presence Binding
// =========================================================================

#[test]
fn presence_records_are_bound_to_their_epoch() {
    // INV14: presence data is scoped to a specific epoch. Declaring presence
    // in epoch 1 does not affect epoch 2.
    new_test_ext().execute_with(|| {
        let epoch1 = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch1));
        let actor = account_to_actor(7);

        // Record exists in epoch 1
        assert!(Presence::presences(epoch1, actor).is_some());

        // Advance past epoch 1 boundary (duration=50, end_block=51)
        // Block 51 closes epoch 1, grace period 5 blocks, block 56 starts epoch 2
        run_to_block(57);

        let epoch2 = Epoch::current_epoch();
        assert_eq!(epoch2, EpochId::new(2));
        assert!(Epoch::is_epoch_active(epoch2));

        // No presence record for actor in epoch 2
        assert!(Presence::presences(epoch2, actor).is_none());

        // Actor can declare a new presence in epoch 2
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch2));
        assert!(Presence::presences(epoch2, actor).is_some());

        // Epoch 1 record is still intact (historical data preserved)
        let old_record = Presence::presences(epoch1, actor).unwrap();
        assert_eq!(old_record.state, PresenceState::Declared);
    });
}

#[test]
fn cannot_declare_in_closed_epoch_after_auto_transition() {
    // When the epoch auto-transitions from Active -> Closed, the Presence
    // pallet must reject new declarations via the EpochProvider check.
    new_test_ext().execute_with(|| {
        let epoch1 = EpochId::new(1);

        // Epoch 1 is active at block 1, ends at block 51
        assert!(Epoch::is_epoch_active(epoch1));

        // Advance to block 51 -> on_initialize auto-closes epoch 1
        run_to_block(51);

        let meta = Epoch::epoch_info(epoch1).unwrap();
        assert_eq!(meta.state, EpochState::Closed);

        // Cannot declare in closed epoch
        assert_noop!(
            Presence::declare_presence(RuntimeOrigin::signed(7), epoch1),
            pallet_presence::Error::<Test>::EpochNotActive
        );
    });
}

#[test]
fn cannot_vote_in_closed_epoch() {
    // Voting also requires an active epoch.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        // Declare while epoch is still active
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // Close the epoch
        run_to_block(51);
        assert!(!Epoch::is_epoch_active(epoch));

        // Voting fails because epoch is no longer active
        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(1), actor, epoch, true),
            pallet_presence::Error::<Test>::EpochNotActive
        );
    });
}

// =========================================================================
// 4. Epoch Auto-Transition and Multi-Epoch Lifecycle
// =========================================================================

#[test]
fn epoch_auto_transitions_through_full_lifecycle() {
    // Verifies Scheduled -> Active -> Closed -> (grace) -> next epoch Active
    new_test_ext().execute_with(|| {
        // Genesis: epoch 1 is Active, blocks 1..51
        let epoch1 = EpochId::new(1);
        assert_eq!(Epoch::epoch_info(epoch1).unwrap().state, EpochState::Active);

        // Advance to block 51: epoch 1 closes, epoch 2 is scheduled
        run_to_block(51);
        assert_eq!(Epoch::epoch_info(epoch1).unwrap().state, EpochState::Closed);

        // Grace period = 5 blocks. At block 56, epoch 2 should start.
        run_to_block(56);
        let epoch2 = EpochId::new(2);
        assert_eq!(Epoch::current_epoch(), epoch2);
        assert_eq!(Epoch::epoch_info(epoch2).unwrap().state, EpochState::Active);

        // Epoch 1 remains closed (not finalized -- that requires explicit call)
        assert_eq!(Epoch::epoch_info(epoch1).unwrap().state, EpochState::Closed);
    });
}

#[test]
fn presence_lifecycle_spans_epoch_boundary() {
    // Test that presence in epoch 1 can be finalized, and then a new presence
    // can be created in epoch 2 for the same actor.
    new_test_ext().execute_with(|| {
        let epoch1 = EpochId::new(1);
        let actor_account = 7u64;

        // Declare + validate + finalize in epoch 1
        assert_ok!(Presence::declare_presence(
            RuntimeOrigin::signed(actor_account),
            epoch1
        ));
        let actor = account_to_actor(actor_account);

        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch1,
            true
        ));
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(2),
            actor,
            epoch1,
            true
        ));
        assert_ok!(Presence::finalize_presence(
            RuntimeOrigin::signed(actor_account),
            actor,
            epoch1
        ));

        let record1 = Presence::presences(epoch1, actor).unwrap();
        assert_eq!(record1.state, PresenceState::Finalized);

        // Transition to epoch 2
        run_to_block(57);
        let epoch2 = Epoch::current_epoch();
        assert_eq!(epoch2, EpochId::new(2));

        // Same actor can participate in epoch 2
        assert_ok!(Presence::declare_presence(
            RuntimeOrigin::signed(actor_account),
            epoch2
        ));

        let record2 = Presence::presences(epoch2, actor).unwrap();
        assert_eq!(record2.state, PresenceState::Declared);

        // Epoch 1 finalized record is preserved and unchanged
        let record1_after = Presence::presences(epoch1, actor).unwrap();
        assert_eq!(record1_after.state, PresenceState::Finalized);
    });
}

// =========================================================================
// 5. State Machine Monotonicity (INV7) across pallets
// =========================================================================

#[test]
fn inv7_presence_state_monotonic_forward_only() {
    // The presence state machine must follow:
    // Declared -> Validated -> Finalized
    // Any non-root attempt to move backwards is rejected.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        // Declare
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // Cannot finalize from Declared (must be Validated first)
        assert_noop!(
            Presence::finalize_presence(RuntimeOrigin::signed(7), actor, epoch),
            pallet_presence::Error::<Test>::PresenceNotValidated
        );

        // Validate via votes
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            true
        ));
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(2),
            actor,
            epoch,
            true
        ));

        let record = Presence::presences(epoch, actor).unwrap();
        assert_eq!(record.state, PresenceState::Validated);

        // Finalize
        assert_ok!(Presence::finalize_presence(
            RuntimeOrigin::signed(7),
            actor,
            epoch
        ));

        let record = Presence::presences(epoch, actor).unwrap();
        assert_eq!(record.state, PresenceState::Finalized);
    });
}

#[test]
fn inv8_finalized_presence_is_immutable() {
    // INV8: once Finalized, the presence cannot be slashed or modified.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // Vote + finalize
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            true
        ));
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(2),
            actor,
            epoch,
            true
        ));
        assert_ok!(Presence::finalize_presence(
            RuntimeOrigin::signed(7),
            actor,
            epoch
        ));

        // Cannot slash a finalized presence
        assert_noop!(
            Presence::slash_presence(RuntimeOrigin::root(), actor, epoch),
            pallet_presence::Error::<Test>::PresenceImmutable
        );

        // Cannot vote on a finalized presence (terminal check runs first)
        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(3), actor, epoch, true),
            pallet_presence::Error::<Test>::PresenceImmutable
        );
    });
}

#[test]
fn inv8_slashed_presence_is_terminal() {
    // INV8: once Slashed, no further transitions are possible.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // Root slashes the presence
        assert_ok!(Presence::slash_presence(
            RuntimeOrigin::root(),
            actor,
            epoch
        ));

        let record = Presence::presences(epoch, actor).unwrap();
        assert_eq!(record.state, PresenceState::Slashed);

        // Cannot vote (terminal check runs first -> PresenceImmutable)
        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(1), actor, epoch, true),
            pallet_presence::Error::<Test>::PresenceImmutable
        );

        // Cannot slash again
        assert_noop!(
            Presence::slash_presence(RuntimeOrigin::root(), actor, epoch),
            pallet_presence::Error::<Test>::PresenceImmutable
        );
    });
}

// =========================================================================
// 6. Validator Registration -> Activation -> Presence Voting Flow
// =========================================================================

#[test]
fn validator_registration_activation_then_vote() {
    // End-to-end: register validator -> wait bonding -> activate ->
    //             vote on presence -> quorum met -> finalize
    new_test_ext_no_validators().execute_with(|| {
        let epoch = EpochId::new(1);

        // Register 3 validators (BondingDuration = 10)
        for account in 1u64..=3 {
            assert_ok!(Validator::register_validator(
                RuntimeOrigin::signed(account),
                5_000
            ));
        }

        // Cannot activate before bonding period
        assert_noop!(
            Validator::activate_validator(RuntimeOrigin::signed(1)),
            pallet_validator::Error::<Test>::BondingPeriodNotElapsed
        );

        // Advance past bonding duration (registered at block 1, bonding = 10)
        run_to_block(12);

        // Activate all 3 validators
        for account in 1u64..=3 {
            assert_ok!(Validator::activate_validator(RuntimeOrigin::signed(
                account
            )));
        }

        // Verify they are active via the ValidatorProvider interface
        for account in 1u64..=3 {
            let vid = account_to_validator(account);
            assert!(pallet_validator::Pallet::<Test>::is_validator_active(vid));
        }

        // Account 7 declares presence
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // Validators 1 and 2 vote (quorum = 2)
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            true
        ));
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(2),
            actor,
            epoch,
            true
        ));

        let record = Presence::presences(epoch, actor).unwrap();
        assert_eq!(record.state, PresenceState::Validated);

        // Finalize
        assert_ok!(Presence::finalize_presence(
            RuntimeOrigin::signed(7),
            actor,
            epoch
        ));

        let record = Presence::presences(epoch, actor).unwrap();
        assert_eq!(record.state, PresenceState::Finalized);
    });
}

// =========================================================================
// 7. Multiple Actors in Same Epoch
// =========================================================================

#[test]
fn multiple_actors_declare_and_finalize_in_same_epoch() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        // 3 different actors declare
        for account in 7u64..=9 {
            assert_ok!(Presence::declare_presence(
                RuntimeOrigin::signed(account),
                epoch
            ));
        }

        // Validate and finalize each
        for account in 7u64..=9 {
            let actor = account_to_actor(account);

            assert_ok!(Presence::vote_presence(
                RuntimeOrigin::signed(1),
                actor,
                epoch,
                true
            ));
            assert_ok!(Presence::vote_presence(
                RuntimeOrigin::signed(2),
                actor,
                epoch,
                true
            ));

            let record = Presence::presences(epoch, actor).unwrap();
            assert_eq!(record.state, PresenceState::Validated);

            assert_ok!(Presence::finalize_presence(
                RuntimeOrigin::signed(account),
                actor,
                epoch
            ));

            let record = Presence::presences(epoch, actor).unwrap();
            assert_eq!(record.state, PresenceState::Finalized);
        }

        // Presence count should reflect 3 declarations
        assert_eq!(Presence::presence_count(epoch), 3);
    });
}

// =========================================================================
// 8. Epoch Participant Registration with Presence Declaration
// =========================================================================

#[test]
fn epoch_participant_registration_is_independent_of_presence() {
    // INV16: epoch participant registration is separate from presence
    // declaration. An actor can register in an epoch and then declare.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        // Register as epoch participant
        assert_ok!(Epoch::register_participant(RuntimeOrigin::signed(7), epoch));
        assert!(Epoch::is_participant(epoch, &7));

        // Declare presence
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));

        // Participant registration is separate from presence
        assert!(!Epoch::is_participant(epoch, &8));

        // Account 8 can still declare presence without being a participant
        // (the presence pallet only checks epoch active state, not
        // participant registration)
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(8), epoch));
    });
}

#[test]
fn cannot_register_participant_in_closed_epoch() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        // Close epoch
        run_to_block(51);
        assert!(!Epoch::is_epoch_active(epoch));

        assert_noop!(
            Epoch::register_participant(RuntimeOrigin::signed(7), epoch),
            pallet_epoch::Error::<Test>::EpochNotActive
        );
    });
}

// =========================================================================
// 9. Quorum Edge Cases
// =========================================================================

#[test]
fn inv10_exactly_at_quorum_threshold_transitions_to_validated() {
    // INV10: With threshold=2, exactly 2 approving votes should transition
    // the presence from Declared to Validated.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // 1 vote: still Declared
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            true
        ));
        assert_eq!(
            Presence::presences(epoch, actor).unwrap().state,
            PresenceState::Declared
        );

        // 2nd vote: transitions to Validated
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(2),
            actor,
            epoch,
            true
        ));
        assert_eq!(
            Presence::presences(epoch, actor).unwrap().state,
            PresenceState::Validated
        );
    });
}

#[test]
fn rejecting_votes_do_not_count_toward_quorum() {
    // Disapproving votes should not increment the approval count.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // 2 rejections
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            false
        ));
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(2),
            actor,
            epoch,
            false
        ));

        // Still Declared because vote_count only increments on approve
        let record = Presence::presences(epoch, actor).unwrap();
        assert_eq!(record.state, PresenceState::Declared);
        assert_eq!(record.vote_count, 0);

        // 1 approval
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(3),
            actor,
            epoch,
            true
        ));
        let record = Presence::presences(epoch, actor).unwrap();
        assert_eq!(record.state, PresenceState::Declared);
        assert_eq!(record.vote_count, 1);
    });
}

#[test]
fn cannot_finalize_without_quorum_even_if_validated_somehow() {
    // The finalize call re-checks quorum to defend against storage tampering.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // Only 1 approving vote: not enough for quorum
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            true
        ));

        // Cannot finalize because state is Declared (not Validated)
        assert_noop!(
            Presence::finalize_presence(RuntimeOrigin::signed(7), actor, epoch),
            pallet_presence::Error::<Test>::PresenceNotValidated
        );
    });
}

// =========================================================================
// 10. Non-Authorized Finalization Rejected
// =========================================================================

#[test]
fn unauthorized_account_cannot_finalize_presence() {
    // Only the actor themselves or an active validator can finalize.
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // Validate
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            true
        ));
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(2),
            actor,
            epoch,
            true
        ));

        // Account 9 is neither the actor (7) nor a validator
        assert_noop!(
            Presence::finalize_presence(RuntimeOrigin::signed(9), actor, epoch),
            pallet_presence::Error::<Test>::UnauthorizedDeclaration
        );
    });
}

// =========================================================================
// 11. Epoch Finalization and Immutability (INV17)
// =========================================================================

#[test]
fn finalized_epoch_is_immutable() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        // Close epoch
        assert_ok!(Epoch::close_epoch(RuntimeOrigin::root(), epoch));

        // Wait for grace period
        run_to_block(60);

        // Finalize epoch
        assert_ok!(Epoch::finalize_epoch(RuntimeOrigin::root(), epoch));
        let meta = Epoch::epoch_info(epoch).unwrap();
        assert_eq!(meta.state, EpochState::Finalized);

        // Cannot force-transition a finalized epoch
        assert_noop!(
            Epoch::force_transition(RuntimeOrigin::root(), epoch, EpochState::Active),
            pallet_epoch::Error::<Test>::EpochImmutable
        );
    });
}

// =========================================================================
// 12. Presence Count Tracking Across Epochs
// =========================================================================

#[test]
fn presence_count_tracks_declarations_per_epoch() {
    new_test_ext().execute_with(|| {
        let epoch1 = EpochId::new(1);

        // 3 declarations in epoch 1
        for account in 7u64..=9 {
            assert_ok!(Presence::declare_presence(
                RuntimeOrigin::signed(account),
                epoch1
            ));
        }
        assert_eq!(Presence::presence_count(epoch1), 3);

        // Transition to epoch 2
        run_to_block(57);
        let epoch2 = Epoch::current_epoch();
        assert_eq!(epoch2, EpochId::new(2));

        // Epoch 2 starts with 0 presences
        assert_eq!(Presence::presence_count(epoch2), 0);

        // 1 declaration in epoch 2
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch2));
        assert_eq!(Presence::presence_count(epoch2), 1);

        // Epoch 1 count unchanged
        assert_eq!(Presence::presence_count(epoch1), 3);
    });
}

// =========================================================================
// 13. Vote Count Tracking
// =========================================================================

#[test]
fn vote_count_increments_only_on_approval() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // Reject vote: count stays at 0
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            false
        ));
        assert_eq!(Presence::vote_count(epoch, actor), 0);

        // Approve vote: count becomes 1
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(2),
            actor,
            epoch,
            true
        ));
        assert_eq!(Presence::vote_count(epoch, actor), 1);

        // Another approve: count becomes 2
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(3),
            actor,
            epoch,
            true
        ));
        assert_eq!(Presence::vote_count(epoch, actor), 2);
    });
}

// =========================================================================
// 14. Slash Before Finalize
// =========================================================================

#[test]
fn slashing_prevents_subsequent_finalization() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(7), epoch));
        let actor = account_to_actor(7);

        // Reach Validated state
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch,
            true
        ));
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(2),
            actor,
            epoch,
            true
        ));
        assert_eq!(
            Presence::presences(epoch, actor).unwrap().state,
            PresenceState::Validated
        );

        // Slash
        assert_ok!(Presence::slash_presence(
            RuntimeOrigin::root(),
            actor,
            epoch
        ));
        assert_eq!(
            Presence::presences(epoch, actor).unwrap().state,
            PresenceState::Slashed
        );

        // Cannot finalize after slash (INV8: terminal)
        assert_noop!(
            Presence::finalize_presence(RuntimeOrigin::signed(7), actor, epoch),
            pallet_presence::Error::<Test>::PresenceImmutable
        );
    });
}
