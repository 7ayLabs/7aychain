#![allow(clippy::disallowed_macros)]

use crate::{self as pallet_carrier, Error, NumberStatus, ProvisioningState, ServiceRequestKind};
use frame_support::{assert_noop, assert_ok, derive_impl, parameter_types, traits::ConstU32};
use frame_system as system;
use seveny_primitives::{
    traits::{
        ActorActivityChecker, CarrierRewardHandler, DeviceEligibilityChecker, EpochProvider,
        PresenceVerifier, ServiceNodePresenceVerifier, ValidatorProvider, ValidatorStakeProvider,
    },
    types::{ActorId, EpochId, ValidatorId},
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

type Block = frame_system::mocking::MockBlock<Test>;

thread_local! {
    static ACTIVE_ACTORS: RefCell<BTreeSet<ActorId>> = const { RefCell::new(BTreeSet::new()) };
    static ACTIVE_DEVICES: RefCell<BTreeSet<(ActorId, u64)>> = const { RefCell::new(BTreeSet::new()) };
    static VERIFIED_PRESENCE: RefCell<BTreeSet<(ActorId, u64)>> = const { RefCell::new(BTreeSet::new()) };
    static VERIFIED_SERVICE_NODE_PRESENCE: RefCell<BTreeSet<(u64, u64)>> = const { RefCell::new(BTreeSet::new()) };
    static ACTIVE_VALIDATORS: RefCell<BTreeSet<ValidatorId>> = const { RefCell::new(BTreeSet::new()) };
    static VALIDATOR_STAKES: RefCell<BTreeMap<ValidatorId, u128>> = const { RefCell::new(BTreeMap::new()) };
    static REWARDS_PAID: RefCell<Vec<(ValidatorId, u128)>> = const { RefCell::new(Vec::new()) };
}

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Carrier: pallet_carrier,
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

pub struct MockEpochProvider;
impl EpochProvider for MockEpochProvider {
    fn is_epoch_active(epoch_id: EpochId) -> bool {
        epoch_id == EpochId::new(1) || epoch_id == EpochId::new(2)
    }

    fn current_epoch() -> EpochId {
        EpochId::new(1)
    }
}

pub struct MockValidatorProvider;
impl ValidatorProvider for MockValidatorProvider {
    fn is_validator_active(validator_id: ValidatorId) -> bool {
        ACTIVE_VALIDATORS.with(|set| set.borrow().contains(&validator_id))
    }
}

pub struct MockActorChecker;
impl ActorActivityChecker for MockActorChecker {
    fn is_actor_active(actor_id: ActorId) -> bool {
        ACTIVE_ACTORS.with(|set| set.borrow().contains(&actor_id))
    }
}

pub struct MockDeviceChecker;
impl DeviceEligibilityChecker for MockDeviceChecker {
    fn is_device_active_for_actor(actor_id: ActorId, device_id: u64) -> bool {
        ACTIVE_DEVICES.with(|set| set.borrow().contains(&(actor_id, device_id)))
    }
}

pub struct MockPresenceVerifier;
impl PresenceVerifier for MockPresenceVerifier {
    fn is_presence_verified(actor_id: ActorId, epoch_id: EpochId) -> bool {
        VERIFIED_PRESENCE.with(|set| set.borrow().contains(&(actor_id, epoch_id.inner())))
    }
}

pub struct MockServiceNodePresenceVerifier;
impl ServiceNodePresenceVerifier<u64> for MockServiceNodePresenceVerifier {
    fn is_service_node_present(controller: &u64, epoch_id: EpochId) -> bool {
        VERIFIED_SERVICE_NODE_PRESENCE
            .with(|set| set.borrow().contains(&(*controller, epoch_id.inner())))
    }
}

pub struct MockValidatorStakeProvider;
impl ValidatorStakeProvider for MockValidatorStakeProvider {
    fn validator_stake(validator_id: ValidatorId) -> u128 {
        VALIDATOR_STAKES.with(|stakes| {
            stakes
                .borrow()
                .get(&validator_id)
                .copied()
                .unwrap_or_default()
        })
    }
}

pub struct MockRewardHandler;
impl CarrierRewardHandler<u64> for MockRewardHandler {
    fn reward_witness(validator: &ValidatorId, amount: u128) {
        REWARDS_PAID.with(|v| v.borrow_mut().push((*validator, amount)));
    }
}

parameter_types! {
    pub const MaxNumbersPerActor: u32 = 2;
    pub const MaxWitnessesPerRequest: u32 = 16;
    pub const WitnessThreshold: u32 = 2;
    pub const ServiceLeaseBlocks: u64 = 50;
    pub const WitnessRewardAmount: u128 = 100_000_000_000;
    pub const MaxRegionsPerServiceNode: u32 = 4;
    pub const MinServiceStake: u128 = 1_000;
    pub const MinRegionalServiceNodes: u32 = 2;
    pub const StrongRegionalServiceNodes: u32 = 3;
    pub const ServiceNodeSlashThreshold: u32 = 2;
    pub const CarrierBridgeAccount: u64 = 99;
}

impl pallet_carrier::Config for Test {
    type WeightInfo = ();
    type EpochProvider = MockEpochProvider;
    type ValidatorProvider = MockValidatorProvider;
    type ValidatorStakeProvider = MockValidatorStakeProvider;
    type ActorChecker = MockActorChecker;
    type DeviceChecker = MockDeviceChecker;
    type PresenceVerifier = MockPresenceVerifier;
    type ServiceNodePresenceVerifier = MockServiceNodePresenceVerifier;
    type RewardHandler = MockRewardHandler;
    type MaxNumbersPerActor = MaxNumbersPerActor;
    type MaxWitnessesPerRequest = MaxWitnessesPerRequest;
    type WitnessThreshold = WitnessThreshold;
    type ServiceLeaseBlocks = ServiceLeaseBlocks;
    type WitnessRewardAmount = WitnessRewardAmount;
    type CarrierBridgeAccount = CarrierBridgeAccount;
    type MaxRegionsPerServiceNode = MaxRegionsPerServiceNode;
    type MinServiceStake = MinServiceStake;
    type MinRegionalServiceNodes = MinRegionalServiceNodes;
    type StrongRegionalServiceNodes = StrongRegionalServiceNodes;
    type ServiceNodeSlashThreshold = ServiceNodeSlashThreshold;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_carrier::GenesisConfig::<Test> {
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
    seveny_primitives::crypto::derive_actor_id(&account.encode())
}

fn account_to_validator(account: u64) -> ValidatorId {
    use parity_scale_codec::Encode;
    seveny_primitives::crypto::derive_validator_id(&account.encode())
}

fn mark_actor_active(account: u64) {
    ACTIVE_ACTORS.with(|set| {
        set.borrow_mut().insert(account_to_actor(account));
    });
}

fn mark_device_active(account: u64, device_id: u64) {
    ACTIVE_DEVICES.with(|set| {
        set.borrow_mut()
            .insert((account_to_actor(account), device_id));
    });
}

fn mark_presence_verified(account: u64, epoch: u64) {
    VERIFIED_PRESENCE.with(|set| {
        set.borrow_mut().insert((account_to_actor(account), epoch));
    });
}

fn mark_service_node_present(account: u64, epoch: u64) {
    VERIFIED_SERVICE_NODE_PRESENCE.with(|set| {
        set.borrow_mut().insert((account, epoch));
    });
}

fn mark_validator_active(account: u64) {
    ACTIVE_VALIDATORS.with(|set| {
        set.borrow_mut().insert(account_to_validator(account));
    });
}

fn mark_validator_stake(account: u64, stake: u128) {
    VALIDATOR_STAKES.with(|stakes| {
        stakes
            .borrow_mut()
            .insert(account_to_validator(account), stake);
    });
}

fn register_service_node(account: u64, region: H256) {
    let regions =
        frame_support::BoundedVec::<H256, MaxRegionsPerServiceNode>::try_from(vec![region])
            .expect("bounded region list");
    assert_ok!(Carrier::register_service_node(
        RuntimeOrigin::signed(account),
        regions
    ));
}

fn prime_service_node(account: u64, region: H256, epoch: u64) {
    mark_validator_active(account);
    mark_validator_stake(account, 10_000);
    mark_service_node_present(account, epoch);
    register_service_node(account, region);
}

fn reset_mock_state() {
    ACTIVE_ACTORS.with(|set| set.borrow_mut().clear());
    ACTIVE_DEVICES.with(|set| set.borrow_mut().clear());
    VERIFIED_PRESENCE.with(|set| set.borrow_mut().clear());
    VERIFIED_SERVICE_NODE_PRESENCE.with(|set| set.borrow_mut().clear());
    ACTIVE_VALIDATORS.with(|set| set.borrow_mut().clear());
    VALIDATOR_STAKES.with(|stakes| stakes.borrow_mut().clear());
    REWARDS_PAID.with(|v| v.borrow_mut().clear());
}

fn get_rewards_paid() -> Vec<(ValidatorId, u128)> {
    REWARDS_PAID.with(|v| v.borrow().clone())
}

#[test]
fn reserve_number_success() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);

        let actor = account_to_actor(1);
        let number = H256::repeat_byte(1);
        let region = H256::repeat_byte(9);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));

        let binding = Carrier::numbers(number).expect("binding exists");
        assert_eq!(binding.owner, actor);
        assert_eq!(binding.status, NumberStatus::Reserved);
        assert_eq!(binding.provisioning_state, ProvisioningState::None);
    });
}

#[test]
fn request_activation_requires_presence_verification() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);
        mark_device_active(1, 7);

        let number = H256::repeat_byte(2);
        let region = H256::repeat_byte(7);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));

        assert_noop!(
            Carrier::request_activation(
                RuntimeOrigin::signed(1),
                number,
                7,
                EpochId::new(1),
                H256::repeat_byte(3)
            ),
            Error::<Test>::PresenceNotVerified
        );
    });
}

#[test]
fn full_activation_flow_succeeds() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);
        mark_device_active(1, 7);
        mark_presence_verified(1, 1);

        let number = H256::repeat_byte(4);
        let region = H256::repeat_byte(8);
        prime_service_node(10, region, 1);
        prime_service_node(11, region, 1);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(1),
            number,
            7,
            EpochId::new(1),
            H256::repeat_byte(5)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(10),
            number,
            EpochId::new(1),
            region,
            80
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(11),
            number,
            EpochId::new(1),
            region,
            85
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(1),
            number,
            EpochId::new(1)
        ));

        let binding = Carrier::numbers(number).expect("binding exists");
        assert_eq!(binding.status, NumberStatus::Activated);
        assert_eq!(binding.current_device_id, Some(7));
        assert_eq!(binding.activation_epoch, Some(EpochId::new(1)));
        assert_eq!(binding.provisioning_state, ProvisioningState::Pending);
        assert_eq!(binding.provisioning_receipt, None);
        assert!(binding.service_lease_until.is_some());
    });
}

#[test]
fn duplicate_service_witness_rejected() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);
        mark_device_active(1, 7);
        mark_presence_verified(1, 1);

        let number = H256::repeat_byte(6);
        let region = H256::repeat_byte(10);
        prime_service_node(10, region, 1);
        prime_service_node(11, region, 1);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(1),
            number,
            7,
            EpochId::new(1),
            H256::repeat_byte(11)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(10),
            number,
            EpochId::new(1),
            region,
            70
        ));

        assert_noop!(
            Carrier::submit_service_witness(
                RuntimeOrigin::signed(10),
                number,
                EpochId::new(1),
                region,
                75
            ),
            Error::<Test>::DuplicateWitness
        );
    });
}

#[test]
fn recovery_flow_rebinds_device() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);
        mark_device_active(1, 7);
        mark_device_active(1, 8);
        mark_presence_verified(1, 1);
        mark_presence_verified(1, 2);

        let number = H256::repeat_byte(12);
        let region = H256::repeat_byte(13);
        prime_service_node(10, region, 1);
        prime_service_node(11, region, 1);
        mark_service_node_present(10, 2);
        mark_service_node_present(11, 2);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(1),
            number,
            7,
            EpochId::new(1),
            H256::repeat_byte(14)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(10),
            number,
            EpochId::new(1),
            region,
            80
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(11),
            number,
            EpochId::new(1),
            region,
            81
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(1),
            number,
            EpochId::new(1)
        ));

        assert_ok!(Carrier::request_recovery(
            RuntimeOrigin::signed(1),
            number,
            8,
            EpochId::new(2),
            H256::repeat_byte(15)
        ));
        let request = Carrier::service_requests(EpochId::new(2), number).expect("request exists");
        assert_eq!(request.kind, ServiceRequestKind::Recovery);

        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(10),
            number,
            EpochId::new(2),
            region,
            90
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(11),
            number,
            EpochId::new(2),
            region,
            91
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(1),
            number,
            EpochId::new(2)
        ));

        let binding = Carrier::numbers(number).expect("binding exists");
        assert_eq!(binding.current_device_id, Some(8));
        assert_eq!(binding.status, NumberStatus::Recovered);
        assert_eq!(binding.activation_epoch, Some(EpochId::new(2)));
        assert_eq!(binding.provisioning_state, ProvisioningState::Pending);
    });
}

#[test]
fn signal_quality_aggregated_on_finalize() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);
        mark_device_active(1, 7);
        mark_presence_verified(1, 1);

        let number = H256::repeat_byte(20);
        let region = H256::repeat_byte(21);
        prime_service_node(10, region, 1);
        prime_service_node(11, region, 1);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(1),
            number,
            7,
            EpochId::new(1),
            H256::repeat_byte(22)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(10),
            number,
            EpochId::new(1),
            region,
            80
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(11),
            number,
            EpochId::new(1),
            region,
            90
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(1),
            number,
            EpochId::new(1)
        ));

        let quality = Carrier::signal_quality(number).expect("quality exists");
        assert_eq!(quality.avg_score, 85);
        assert_eq!(quality.min_score, 80);
        assert_eq!(quality.max_score, 90);
        assert_eq!(quality.sample_count, 2);
    });
}

#[test]
fn witness_rewards_paid_on_finalize() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);
        mark_device_active(1, 7);
        mark_presence_verified(1, 1);

        let number = H256::repeat_byte(30);
        let region = H256::repeat_byte(31);
        prime_service_node(10, region, 1);
        prime_service_node(11, region, 1);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(1),
            number,
            7,
            EpochId::new(1),
            H256::repeat_byte(32)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(10),
            number,
            EpochId::new(1),
            region,
            80
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(11),
            number,
            EpochId::new(1),
            region,
            90
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(1),
            number,
            EpochId::new(1)
        ));

        let rewards = get_rewards_paid();
        assert_eq!(rewards.len(), 2);
        // Both validators received the reward amount
        for (_validator, amount) in &rewards {
            assert_eq!(*amount, 100_000_000_000);
        }

        // Witness count incremented for both validators
        let v10 = account_to_validator(10);
        let v11 = account_to_validator(11);
        assert_eq!(Carrier::validator_witness_count(v10), 1);
        assert_eq!(Carrier::validator_witness_count(v11), 1);
    });
}

#[test]
fn refresh_signal_quality_works() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);
        mark_device_active(1, 7);
        mark_presence_verified(1, 1);

        let number = H256::repeat_byte(40);
        let region = H256::repeat_byte(41);
        prime_service_node(10, region, 1);
        prime_service_node(11, region, 1);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(1),
            number,
            7,
            EpochId::new(1),
            H256::repeat_byte(42)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(10),
            number,
            EpochId::new(1),
            region,
            70
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(11),
            number,
            EpochId::new(1),
            region,
            90
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(1),
            number,
            EpochId::new(1)
        ));

        // After finalize, witnesses are cleared but quality persists
        let quality = Carrier::signal_quality(number).expect("quality exists");
        assert_eq!(quality.avg_score, 80);

        // refresh_signal_quality on a number without active witnesses
        // still works (no-op since witnesses were cleared)
        assert_ok!(Carrier::refresh_signal_quality(
            RuntimeOrigin::signed(1),
            number,
            EpochId::new(1)
        ));
    });
}

#[test]
fn provisioning_result_requires_bridge_account() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);
        mark_device_active(1, 7);
        mark_presence_verified(1, 1);

        let number = H256::repeat_byte(50);
        let region = H256::repeat_byte(51);
        prime_service_node(10, region, 1);
        prime_service_node(11, region, 1);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(1),
            number,
            7,
            EpochId::new(1),
            H256::repeat_byte(52)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(10),
            number,
            EpochId::new(1),
            region,
            80
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(11),
            number,
            EpochId::new(1),
            region,
            90
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(1),
            number,
            EpochId::new(1)
        ));

        assert_noop!(
            Carrier::record_provisioning_result(
                RuntimeOrigin::signed(1),
                number,
                H256::repeat_byte(53),
                true
            ),
            Error::<Test>::ProvisioningUpdateNotAllowed
        );
    });
}

#[test]
fn bridge_can_record_provisioning_success_and_failure() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);
        mark_device_active(1, 7);
        mark_presence_verified(1, 1);

        let number = H256::repeat_byte(60);
        let region = H256::repeat_byte(61);
        prime_service_node(10, region, 1);
        prime_service_node(11, region, 1);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(1),
            number,
            7,
            EpochId::new(1),
            H256::repeat_byte(62)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(10),
            number,
            EpochId::new(1),
            region,
            80
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(11),
            number,
            EpochId::new(1),
            region,
            90
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(1),
            number,
            EpochId::new(1)
        ));

        assert_ok!(Carrier::record_provisioning_result(
            RuntimeOrigin::signed(99),
            number,
            H256::repeat_byte(63),
            false
        ));

        let failed = Carrier::numbers(number).expect("binding exists");
        assert_eq!(failed.provisioning_state, ProvisioningState::Failed);
        assert_eq!(failed.provisioning_receipt, Some(H256::repeat_byte(63)));

        assert_ok!(Carrier::record_provisioning_result(
            RuntimeOrigin::signed(99),
            number,
            H256::repeat_byte(64),
            true
        ));

        let provisioned = Carrier::numbers(number).expect("binding exists");
        assert_eq!(
            provisioned.provisioning_state,
            ProvisioningState::Provisioned
        );
        assert_eq!(
            provisioned.provisioning_receipt,
            Some(H256::repeat_byte(64))
        );
    });
}

#[test]
fn activation_requires_regional_service_coverage() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);
        mark_device_active(1, 7);
        mark_presence_verified(1, 1);

        let number = H256::repeat_byte(70);
        let region = H256::repeat_byte(71);
        prime_service_node(10, region, 1);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));
        assert_noop!(
            Carrier::request_activation(
                RuntimeOrigin::signed(1),
                number,
                7,
                EpochId::new(1),
                H256::repeat_byte(72)
            ),
            Error::<Test>::InsufficientRegionalCoverage
        );

        prime_service_node(11, region, 1);
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(1),
            number,
            7,
            EpochId::new(1),
            H256::repeat_byte(72)
        ));
    });
}

#[test]
fn failure_reports_suspend_service_node_after_threshold() {
    new_test_ext().execute_with(|| {
        reset_mock_state();
        mark_actor_active(1);
        mark_device_active(1, 7);
        mark_device_active(1, 8);
        mark_presence_verified(1, 1);

        let number = H256::repeat_byte(80);
        let second_number = H256::repeat_byte(81);
        let region = H256::repeat_byte(82);
        prime_service_node(10, region, 1);
        prime_service_node(11, region, 1);

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            number,
            region
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(1),
            number,
            7,
            EpochId::new(1),
            H256::repeat_byte(83)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(10),
            number,
            EpochId::new(1),
            region,
            88
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(11),
            number,
            EpochId::new(1),
            region,
            90
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(1),
            number,
            EpochId::new(1)
        ));
        assert_ok!(Carrier::report_service_failure(
            RuntimeOrigin::signed(1),
            number,
            account_to_validator(10)
        ));

        assert_ok!(Carrier::reserve_number(
            RuntimeOrigin::signed(1),
            second_number,
            region
        ));
        assert_ok!(Carrier::request_activation(
            RuntimeOrigin::signed(1),
            second_number,
            8,
            EpochId::new(1),
            H256::repeat_byte(84)
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(10),
            second_number,
            EpochId::new(1),
            region,
            86
        ));
        assert_ok!(Carrier::submit_service_witness(
            RuntimeOrigin::signed(11),
            second_number,
            EpochId::new(1),
            region,
            87
        ));
        assert_ok!(Carrier::finalize_service_request(
            RuntimeOrigin::signed(1),
            second_number,
            EpochId::new(1)
        ));
        assert_ok!(Carrier::report_service_failure(
            RuntimeOrigin::signed(1),
            second_number,
            account_to_validator(10)
        ));

        let profile = Carrier::service_nodes(account_to_validator(10)).expect("node exists");
        assert_eq!(profile.status, crate::ServiceNodeStatus::Suspended);
        assert_eq!(profile.slash_points, 2);

        let coverage = Carrier::region_coverage(EpochId::new(1), region).expect("coverage exists");
        assert_eq!(coverage.eligible_node_count, 1);
    });
}
