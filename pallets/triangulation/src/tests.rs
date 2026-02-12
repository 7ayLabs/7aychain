#![allow(clippy::disallowed_macros)]

use crate::{
    self as pallet_triangulation, DeviceState, Error, Position, ReporterId, SignalType,
};
use frame_support::{assert_noop, assert_ok, derive_impl, parameter_types, traits::ConstU32};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Triangulation: pallet_triangulation,
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
    pub const MaxReporters: u32 = 100;
    pub const MaxReadingsPerDevice: u32 = 50;
    pub const MaxHistoryEntries: u32 = 1000;
    pub const InactiveTimeoutBlocks: u64 = 10;
    pub const LostTimeoutBlocks: u64 = 100;
    pub const MinReadingsForActive: u32 = 3;
    pub const SignalRetentionBlocks: u64 = 1000;
}

impl pallet_triangulation::Config for Test {
    type WeightInfo = ();
    type MaxReporters = MaxReporters;
    type MaxReadingsPerDevice = MaxReadingsPerDevice;
    type MaxHistoryEntries = MaxHistoryEntries;
    type InactiveTimeoutBlocks = InactiveTimeoutBlocks;
    type LostTimeoutBlocks = LostTimeoutBlocks;
    type MinReadingsForActive = MinReadingsForActive;
    type SignalRetentionBlocks = SignalRetentionBlocks;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_triangulation::GenesisConfig::<Test> {
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("genesis build failed");

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn register_reporter_success() {
    new_test_ext().execute_with(|| {
        let position = Position { x: 100, y: 200, z: 0 };

        assert_ok!(Triangulation::register_reporter(
            RuntimeOrigin::signed(1),
            position.clone()
        ));

        let reporter = Triangulation::reporters(ReporterId::new(0)).expect("reporter should exist");
        assert_eq!(reporter.position, position);
        assert!(reporter.active);
        assert_eq!(Triangulation::reporter_count(), 1);
    });
}

#[test]
fn deregister_reporter_success() {
    new_test_ext().execute_with(|| {
        let position = Position { x: 100, y: 200, z: 0 };

        assert_ok!(Triangulation::register_reporter(
            RuntimeOrigin::signed(1),
            position
        ));

        assert_ok!(Triangulation::deregister_reporter(
            RuntimeOrigin::signed(1),
            ReporterId::new(0)
        ));

        let reporter = Triangulation::reporters(ReporterId::new(0)).expect("reporter should exist");
        assert!(!reporter.active);
    });
}

#[test]
fn report_signal_creates_device() {
    new_test_ext().execute_with(|| {
        let position = Position { x: 100, y: 200, z: 0 };
        let mac_hash = H256([1u8; 32]);

        assert_ok!(Triangulation::register_reporter(
            RuntimeOrigin::signed(1),
            position
        ));

        assert_ok!(Triangulation::report_signal(
            RuntimeOrigin::signed(1),
            ReporterId::new(0),
            mac_hash,
            -50,
            SignalType::Wifi,
            2400
        ));

        let device = Triangulation::tracked_devices(mac_hash).expect("device should exist");
        assert_eq!(device.signal_type, SignalType::Wifi);
        assert_eq!(device.state, DeviceState::Active);
        assert_eq!(Triangulation::device_count(), 1);
    });
}

#[test]
fn invalid_rssi_rejected() {
    new_test_ext().execute_with(|| {
        let position = Position { x: 100, y: 200, z: 0 };
        let mac_hash = H256([1u8; 32]);

        assert_ok!(Triangulation::register_reporter(
            RuntimeOrigin::signed(1),
            position
        ));

        assert_noop!(
            Triangulation::report_signal(
                RuntimeOrigin::signed(1),
                ReporterId::new(0),
                mac_hash,
                10,
                SignalType::Wifi,
                2400
            ),
            Error::<Test>::InvalidRssi
        );
    });
}

#[test]
fn inactive_reporter_cannot_report() {
    new_test_ext().execute_with(|| {
        let position = Position { x: 100, y: 200, z: 0 };
        let mac_hash = H256([1u8; 32]);

        assert_ok!(Triangulation::register_reporter(
            RuntimeOrigin::signed(1),
            position
        ));

        assert_ok!(Triangulation::deregister_reporter(
            RuntimeOrigin::signed(1),
            ReporterId::new(0)
        ));

        assert_noop!(
            Triangulation::report_signal(
                RuntimeOrigin::signed(1),
                ReporterId::new(0),
                mac_hash,
                -50,
                SignalType::Wifi,
                2400
            ),
            Error::<Test>::ReporterNotActive
        );
    });
}

#[test]
fn signal_history_stored() {
    new_test_ext().execute_with(|| {
        let position = Position { x: 100, y: 200, z: 0 };
        let mac_hash = H256([1u8; 32]);

        assert_ok!(Triangulation::register_reporter(
            RuntimeOrigin::signed(1),
            position
        ));

        assert_ok!(Triangulation::report_signal(
            RuntimeOrigin::signed(1),
            ReporterId::new(0),
            mac_hash,
            -50,
            SignalType::Wifi,
            2400
        ));

        let history = Triangulation::get_device_history(mac_hash);
        assert_eq!(history.len(), 1);
    });
}

#[test]
fn update_reporter_position_success() {
    new_test_ext().execute_with(|| {
        let position = Position { x: 100, y: 200, z: 0 };
        let new_position = Position { x: 300, y: 400, z: 10 };

        assert_ok!(Triangulation::register_reporter(
            RuntimeOrigin::signed(1),
            position
        ));

        assert_ok!(Triangulation::update_reporter_position(
            RuntimeOrigin::signed(1),
            ReporterId::new(0),
            new_position.clone()
        ));

        let reporter = Triangulation::reporters(ReporterId::new(0)).expect("reporter should exist");
        assert_eq!(reporter.position, new_position);
    });
}

#[test]
fn genesis_initializes_counts() {
    new_test_ext().execute_with(|| {
        assert_eq!(Triangulation::reporter_count(), 0);
        assert_eq!(Triangulation::device_count(), 0);
        assert_eq!(Triangulation::active_device_count(), 0);
        assert_eq!(Triangulation::ghost_count(), 0);
    });
}

#[test]
fn multiple_signals_improve_confidence() {
    new_test_ext().execute_with(|| {
        let position = Position { x: 100, y: 200, z: 0 };
        let mac_hash = H256([1u8; 32]);

        assert_ok!(Triangulation::register_reporter(
            RuntimeOrigin::signed(1),
            position
        ));

        for _ in 0..5 {
            assert_ok!(Triangulation::report_signal(
                RuntimeOrigin::signed(1),
                ReporterId::new(0),
                mac_hash,
                -50,
                SignalType::Wifi,
                2400
            ));
        }

        let device = Triangulation::tracked_devices(mac_hash).expect("device should exist");
        assert!(device.confidence > 30);
        assert_eq!(device.reading_count, 5);
    });
}

#[test]
fn all_signal_types() {
    new_test_ext().execute_with(|| {
        let position = Position { x: 100, y: 200, z: 0 };

        assert_ok!(Triangulation::register_reporter(
            RuntimeOrigin::signed(1),
            position
        ));

        let signal_types = [
            SignalType::Wifi,
            SignalType::Bluetooth,
            SignalType::Ble,
            SignalType::Zigbee,
            SignalType::Unknown,
        ];

        for (i, signal_type) in signal_types.iter().enumerate() {
            let mac_hash = H256([i as u8; 32]);

            assert_ok!(Triangulation::report_signal(
                RuntimeOrigin::signed(1),
                ReporterId::new(0),
                mac_hash,
                -50,
                *signal_type,
                2400
            ));

            let device = Triangulation::tracked_devices(mac_hash).expect("device should exist");
            assert_eq!(device.signal_type, *signal_type);
        }

        assert_eq!(Triangulation::device_count(), 5);
    });
}

