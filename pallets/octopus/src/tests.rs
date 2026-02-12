#![allow(clippy::disallowed_macros)]

use crate::{
    self as pallet_octopus, ClusterId, ClusterStatus, Error, Event, ScalingDecision, SubnodeId,
    SubnodeStatus,
};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, Hooks},
};
use frame_system as system;
use seveny_primitives::types::ActorId;
use sp_arithmetic::Perbill;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Octopus: pallet_octopus,
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
    pub const ActivationThreshold: Perbill = Perbill::from_percent(45);
    pub const DeactivationThreshold: Perbill = Perbill::from_percent(20);
    pub const DeactivationDurationBlocks: u64 = 50;
    pub const MaxSubnodesPerCluster: u32 = 8;
    pub const MinSubnodes: u32 = 1;
    pub const ScalingCooldownBlocks: u64 = 10;
    pub const HeartbeatTimeoutBlocks: u64 = 10;
    pub const MaxConsecutiveMisses: u8 = 3;
    pub const HealthScoreDecay: u8 = 10;
    pub const HealthScoreRecovery: u8 = 5;
}

impl pallet_octopus::Config for Test {
    type WeightInfo = ();
    type ActivationThreshold = ActivationThreshold;
    type DeactivationThreshold = DeactivationThreshold;
    type DeactivationDurationBlocks = DeactivationDurationBlocks;
    type MaxSubnodesPerCluster = MaxSubnodesPerCluster;
    type MinSubnodes = MinSubnodes;
    type ScalingCooldownBlocks = ScalingCooldownBlocks;
    type HeartbeatTimeoutBlocks = HeartbeatTimeoutBlocks;
    type MaxConsecutiveMisses = MaxConsecutiveMisses;
    type HealthScoreDecay = HealthScoreDecay;
    type HealthScoreRecovery = HealthScoreRecovery;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_octopus::GenesisConfig::<Test> {
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
fn create_cluster_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));

        let cluster_id = ClusterId::new(0);
        let cluster = Octopus::clusters(cluster_id).expect("cluster should exist");

        assert_eq!(cluster.owner, owner);
        assert_eq!(cluster.status, ClusterStatus::Initializing);
        assert_eq!(cluster.active_subnodes, 0);
        assert_eq!(cluster.max_subnodes, 8);
    });
}

#[test]
fn register_subnode_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let operator = account_to_actor(2);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));

        let cluster_id = ClusterId::new(0);

        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            cluster_id,
            operator
        ));

        let subnode_id = SubnodeId::new(0);
        let subnode = Octopus::subnodes(subnode_id).expect("subnode should exist");

        assert_eq!(subnode.operator, operator);
        assert_eq!(subnode.cluster, cluster_id);
        assert_eq!(subnode.status, SubnodeStatus::Inactive);
    });
}

#[test]
fn activate_subnode_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let operator = account_to_actor(2);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator
        ));

        let subnode_id = SubnodeId::new(0);
        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(2),
            subnode_id
        ));

        let subnode = Octopus::subnodes(subnode_id).expect("subnode should exist");
        assert_eq!(subnode.status, SubnodeStatus::Active);
        assert!(subnode.activated_at.is_some());

        let cluster = Octopus::clusters(ClusterId::new(0)).expect("cluster should exist");
        assert_eq!(cluster.active_subnodes, 1);
        assert_eq!(cluster.status, ClusterStatus::Running);

        assert_eq!(Octopus::get_total_active_subnodes(), 1);
    });
}

#[test]
fn cannot_activate_already_active() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let operator = account_to_actor(2);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator
        ));

        let subnode_id = SubnodeId::new(0);
        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(2),
            subnode_id
        ));

        assert_noop!(
            Octopus::activate_subnode(RuntimeOrigin::signed(2), subnode_id),
            Error::<Test>::SubnodeAlreadyActive
        );
    });
}

#[test]
fn start_deactivation_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let operator1 = account_to_actor(2);
        let operator2 = account_to_actor(3);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator1
        ));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator2
        ));

        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(2),
            SubnodeId::new(0)
        ));
        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(3),
            SubnodeId::new(1)
        ));

        assert_ok!(Octopus::start_deactivation(
            RuntimeOrigin::signed(2),
            SubnodeId::new(0)
        ));

        let subnode = Octopus::subnodes(SubnodeId::new(0)).expect("subnode should exist");
        assert_eq!(subnode.status, SubnodeStatus::Deactivating);
        assert!(subnode.deactivation_started.is_some());
    });
}

#[test]
fn cannot_deactivate_below_minimum() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let operator = account_to_actor(2);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator
        ));
        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(2),
            SubnodeId::new(0)
        ));

        assert_noop!(
            Octopus::start_deactivation(RuntimeOrigin::signed(2), SubnodeId::new(0)),
            Error::<Test>::MinSubnodesRequired
        );
    });
}

#[test]
fn deactivation_completes_after_duration() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let operator1 = account_to_actor(2);
        let operator2 = account_to_actor(3);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator1
        ));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator2
        ));

        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(2),
            SubnodeId::new(0)
        ));
        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(3),
            SubnodeId::new(1)
        ));

        assert_ok!(Octopus::start_deactivation(
            RuntimeOrigin::signed(2),
            SubnodeId::new(0)
        ));

        System::set_block_number(52);
        Octopus::on_initialize(52);

        let subnode = Octopus::subnodes(SubnodeId::new(0)).expect("subnode should exist");
        assert_eq!(subnode.status, SubnodeStatus::Inactive);

        let cluster = Octopus::clusters(ClusterId::new(0)).expect("cluster should exist");
        assert_eq!(cluster.active_subnodes, 1);

        assert_eq!(Octopus::get_total_active_subnodes(), 1);
    });
}

#[test]
fn update_throughput_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));

        let cluster_id = ClusterId::new(0);

        assert_ok!(Octopus::update_throughput(
            RuntimeOrigin::root(),
            cluster_id,
            Perbill::from_percent(50)
        ));

        let cluster = Octopus::clusters(cluster_id).expect("cluster should exist");
        assert_eq!(cluster.total_throughput, Perbill::from_percent(50));

        let metric = Octopus::throughput_history(cluster_id).expect("metric should exist");
        assert_eq!(metric.throughput, Perbill::from_percent(50));
        assert_eq!(metric.sample_count, 1);
    });
}

#[test]
fn scaling_decision_scale_up() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let operator = account_to_actor(2);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator
        ));
        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(2),
            SubnodeId::new(0)
        ));

        assert_ok!(Octopus::update_throughput(
            RuntimeOrigin::root(),
            ClusterId::new(0),
            Perbill::from_percent(50)
        ));

        System::set_block_number(15);

        assert_ok!(Octopus::evaluate_scaling(
            RuntimeOrigin::signed(1),
            ClusterId::new(0)
        ));

        let decision = Octopus::is_scaling_needed(ClusterId::new(0));
        assert!(matches!(decision, Some(ScalingDecision::ScaleUp(_))));
    });
}

#[test]
fn scaling_decision_scale_down() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let operator1 = account_to_actor(2);
        let operator2 = account_to_actor(3);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator1
        ));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator2
        ));

        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(2),
            SubnodeId::new(0)
        ));
        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(3),
            SubnodeId::new(1)
        ));

        assert_ok!(Octopus::update_throughput(
            RuntimeOrigin::root(),
            ClusterId::new(0),
            Perbill::from_percent(15)
        ));

        let decision = Octopus::is_scaling_needed(ClusterId::new(0));
        assert_eq!(decision, Some(ScalingDecision::ScaleDown));
    });
}

#[test]
fn scaling_cooldown_enforced() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let operator = account_to_actor(2);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator
        ));
        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(2),
            SubnodeId::new(0)
        ));

        assert_ok!(Octopus::update_throughput(
            RuntimeOrigin::root(),
            ClusterId::new(0),
            Perbill::from_percent(50)
        ));

        System::set_block_number(15);
        assert_ok!(Octopus::evaluate_scaling(
            RuntimeOrigin::signed(1),
            ClusterId::new(0)
        ));

        System::set_block_number(20);
        assert_noop!(
            Octopus::evaluate_scaling(RuntimeOrigin::signed(1), ClusterId::new(0)),
            Error::<Test>::ScalingCooldownActive
        );
    });
}

#[test]
fn max_subnodes_enforced() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));

        for i in 0..8 {
            assert_ok!(Octopus::register_subnode(
                RuntimeOrigin::signed(1),
                ClusterId::new(0),
                account_to_actor(i + 10)
            ));
        }

        assert_noop!(
            Octopus::register_subnode(
                RuntimeOrigin::signed(1),
                ClusterId::new(0),
                account_to_actor(100)
            ),
            Error::<Test>::MaxSubnodesReached
        );
    });
}

#[test]
fn get_cluster_subnodes_helper() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));

        for i in 0..3 {
            assert_ok!(Octopus::register_subnode(
                RuntimeOrigin::signed(1),
                ClusterId::new(0),
                account_to_actor(i + 10)
            ));
        }

        let subnodes = Octopus::get_cluster_subnodes(ClusterId::new(0));
        assert_eq!(subnodes.len(), 3);
    });
}

#[test]
fn get_active_subnodes_helper() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));

        for i in 0..3 {
            assert_ok!(Octopus::register_subnode(
                RuntimeOrigin::signed(1),
                ClusterId::new(0),
                account_to_actor(i + 10)
            ));
        }

        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(10),
            SubnodeId::new(0)
        ));
        assert_ok!(Octopus::activate_subnode(
            RuntimeOrigin::signed(11),
            SubnodeId::new(1)
        ));

        let active = Octopus::get_active_subnodes(ClusterId::new(0));
        assert_eq!(active.len(), 2);
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));

        System::assert_has_event(RuntimeEvent::Octopus(Event::ClusterCreated {
            cluster_id: ClusterId::new(0),
            owner,
        }));
    });
}

#[test]
fn genesis_initializes_counts() {
    new_test_ext().execute_with(|| {
        assert_eq!(Octopus::subnode_count(), 0);
        assert_eq!(Octopus::cluster_count(), 0);
        assert_eq!(Octopus::get_total_active_subnodes(), 0);
    });
}

#[test]
fn update_subnode_throughput_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let operator = account_to_actor(2);

        assert_ok!(Octopus::create_cluster(RuntimeOrigin::signed(1), owner));
        assert_ok!(Octopus::register_subnode(
            RuntimeOrigin::signed(1),
            ClusterId::new(0),
            operator
        ));

        assert_ok!(Octopus::update_subnode_throughput(
            RuntimeOrigin::signed(2),
            SubnodeId::new(0),
            Perbill::from_percent(75),
            1000
        ));

        let subnode = Octopus::subnodes(SubnodeId::new(0)).expect("subnode should exist");
        assert_eq!(subnode.throughput, Perbill::from_percent(75));
        assert_eq!(subnode.processed_count, 1000);
    });
}
