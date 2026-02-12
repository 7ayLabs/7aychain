#[cfg(test)]
mod tests {
    use crate::*;
    use frame_support::{
        assert_noop, assert_ok, derive_impl, parameter_types,
        traits::ConstU32,
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
            DeviceScanner: crate,
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
        pub const MaxTrackedDevices: u32 = 1000;
        pub const DeviceStaleBlocks: u64 = 50;
        pub const MaxHistoryPerDevice: u32 = 10;
    }

    impl crate::pallet::Config for Test {
        type RuntimeEvent = RuntimeEvent;
        type WeightInfo = ();
        type MaxTrackedDevices = MaxTrackedDevices;
        type DeviceStaleBlocks = DeviceStaleBlocks;
        type MaxHistoryPerDevice = MaxHistoryPerDevice;
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();
        t.into()
    }

    fn make_device(mac: u8, rssi: i8, device_type: DetectedDeviceType) -> ScannedDevice {
        ScannedDevice {
            mac_hash: H256::repeat_byte(mac),
            rssi,
            signal_type: ScanSignalType::Wifi,
            device_type,
            vendor_hash: None,
            name_hash: None,
            frequency: Some(2412),
        }
    }

    fn make_scan_data(devices: Vec<ScannedDevice>) -> DeviceScanInherentData {
        DeviceScanInherentData {
            devices,
            reporter_position: Position { x: 100, y: 200, z: 0 },
            scan_timestamp: 1000,
        }
    }

    #[test]
    fn test_set_scan_data_success() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let device = make_device(0x01, -50, DetectedDeviceType::IPhone);
            let data = make_scan_data(vec![device]);

            assert_ok!(DeviceScanner::set_scan_data(RuntimeOrigin::none(), data));
            assert_eq!(DeviceScanner::device_count(), 1);
            assert_eq!(DeviceScanner::active_device_count(), 1);
        });
    }

    #[test]
    fn test_set_scan_data_max_devices() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let devices: Vec<ScannedDevice> = (0..=MAX_DEVICES_PER_INHERENT as u8)
                .map(|i| make_device(i, -50, DetectedDeviceType::Unknown))
                .collect();

            let data = make_scan_data(devices);

            assert_noop!(
                DeviceScanner::set_scan_data(RuntimeOrigin::none(), data),
                Error::<Test>::TooManyDevices
            );
        });
    }

    #[test]
    fn test_set_scan_data_invalid_origin() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let device = make_device(0x01, -50, DetectedDeviceType::IPhone);
            let data = make_scan_data(vec![device]);

            assert_noop!(
                DeviceScanner::set_scan_data(RuntimeOrigin::signed(1), data),
                frame_support::error::BadOrigin
            );
        });
    }

    #[test]
    fn test_device_first_seen_recorded() {
        new_test_ext().execute_with(|| {
            System::set_block_number(5);

            let device = make_device(0x01, -50, DetectedDeviceType::Android);
            let data = make_scan_data(vec![device.clone()]);

            assert_ok!(DeviceScanner::set_scan_data(RuntimeOrigin::none(), data));

            let tracked = DeviceScanner::tracked_devices(device.mac_hash).unwrap();
            assert_eq!(tracked.first_seen, 5);
        });
    }

    #[test]
    fn test_device_last_seen_updated() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            let device = make_device(0x01, -50, DetectedDeviceType::MacBook);

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(vec![device.clone()])
            ));

            System::set_block_number(10);
            crate::pallet::ScanDataReceived::<Test>::put(false);

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(vec![device.clone()])
            ));

            let tracked = DeviceScanner::tracked_devices(device.mac_hash).unwrap();
            assert_eq!(tracked.first_seen, 1);
            assert_eq!(tracked.last_seen, 10);
        });
    }

    #[test]
    fn test_device_detection_count_increments() {
        new_test_ext().execute_with(|| {
            let device = make_device(0x01, -50, DetectedDeviceType::IPhone);

            for block in 1..=5 {
                System::set_block_number(block);
                crate::pallet::ScanDataReceived::<Test>::put(false);

                assert_ok!(DeviceScanner::set_scan_data(
                    RuntimeOrigin::none(),
                    make_scan_data(vec![device.clone()])
                ));
            }

            let tracked = DeviceScanner::tracked_devices(device.mac_hash).unwrap();
            assert_eq!(tracked.detection_count, 5);
        });
    }

    #[test]
    fn test_device_rssi_updated() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            let device = make_device(0x01, -70, DetectedDeviceType::Android);

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(vec![device.clone()])
            ));

            let tracked = DeviceScanner::tracked_devices(device.mac_hash).unwrap();
            assert_eq!(tracked.last_rssi, -70);

            System::set_block_number(2);
            crate::pallet::ScanDataReceived::<Test>::put(false);

            let updated_device = make_device(0x01, -40, DetectedDeviceType::Android);
            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(vec![updated_device])
            ));

            let tracked = DeviceScanner::tracked_devices(device.mac_hash).unwrap();
            assert_eq!(tracked.last_rssi, -40);
        });
    }

    #[test]
    fn test_multiple_devices_single_scan() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let devices = vec![
                make_device(0x01, -50, DetectedDeviceType::IPhone),
                make_device(0x02, -60, DetectedDeviceType::Android),
                make_device(0x03, -70, DetectedDeviceType::MacBook),
            ];

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(devices)
            ));

            assert_eq!(DeviceScanner::device_count(), 3);
            assert_eq!(DeviceScanner::active_device_count(), 3);
        });
    }

    #[test]
    fn test_device_type_count_increments() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let devices = vec![
                make_device(0x01, -50, DetectedDeviceType::IPhone),
                make_device(0x02, -60, DetectedDeviceType::IPhone),
                make_device(0x03, -70, DetectedDeviceType::Android),
            ];

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(devices)
            ));

            assert_eq!(DeviceScanner::device_type_count(DetectedDeviceType::IPhone), 2);
            assert_eq!(DeviceScanner::device_type_count(DetectedDeviceType::Android), 1);
            assert_eq!(DeviceScanner::device_type_count(DetectedDeviceType::MacBook), 0);
        });
    }

    #[test]
    fn test_scan_data_already_received() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let device = make_device(0x01, -50, DetectedDeviceType::IPhone);
            let data = make_scan_data(vec![device.clone()]);

            assert_ok!(DeviceScanner::set_scan_data(RuntimeOrigin::none(), data.clone()));

            assert_noop!(
                DeviceScanner::set_scan_data(RuntimeOrigin::none(), data),
                Error::<Test>::ScanDataAlreadyReceived
            );
        });
    }

    #[test]
    fn test_scan_timestamp_stored() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let mut data = make_scan_data(vec![make_device(0x01, -50, DetectedDeviceType::IPhone)]);
            data.scan_timestamp = 12345;

            assert_ok!(DeviceScanner::set_scan_data(RuntimeOrigin::none(), data));
            assert_eq!(DeviceScanner::last_scan_timestamp(), 12345);
        });
    }

    #[test]
    fn test_get_statistics() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let devices = vec![
                make_device(0x01, -50, DetectedDeviceType::IPhone),
                make_device(0x02, -60, DetectedDeviceType::Android),
            ];
            let mut data = make_scan_data(devices);
            data.scan_timestamp = 9999;

            assert_ok!(DeviceScanner::set_scan_data(RuntimeOrigin::none(), data));

            let (total, active, timestamp) = DeviceScanner::get_statistics();
            assert_eq!(total, 2);
            assert_eq!(active, 2);
            assert_eq!(timestamp, 9999);
        });
    }

    #[test]
    fn test_get_devices_by_type() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let devices = vec![
                make_device(0x01, -50, DetectedDeviceType::IPhone),
                make_device(0x02, -60, DetectedDeviceType::IPhone),
                make_device(0x03, -70, DetectedDeviceType::Android),
            ];

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(devices)
            ));

            let iphones = DeviceScanner::get_devices_by_type(DetectedDeviceType::IPhone);
            assert_eq!(iphones.len(), 2);

            let androids = DeviceScanner::get_devices_by_type(DetectedDeviceType::Android);
            assert_eq!(androids.len(), 1);

            let macbooks = DeviceScanner::get_devices_by_type(DetectedDeviceType::MacBook);
            assert_eq!(macbooks.len(), 0);
        });
    }

    #[test]
    fn test_empty_scan_data() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let data = make_scan_data(vec![]);

            assert_ok!(DeviceScanner::set_scan_data(RuntimeOrigin::none(), data));
            assert_eq!(DeviceScanner::device_count(), 0);
        });
    }

    #[test]
    fn test_position_recorded() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let device = make_device(0x01, -50, DetectedDeviceType::IPhone);
            let mut data = make_scan_data(vec![device.clone()]);
            data.reporter_position = Position { x: 500, y: 600, z: 700 };

            assert_ok!(DeviceScanner::set_scan_data(RuntimeOrigin::none(), data));

            let tracked = DeviceScanner::tracked_devices(device.mac_hash).unwrap();
            assert_eq!(tracked.last_position.x, 500);
            assert_eq!(tracked.last_position.y, 600);
            assert_eq!(tracked.last_position.z, 700);
        });
    }

    #[test]
    fn test_wifi_signal_processing() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let device = ScannedDevice {
                mac_hash: H256::repeat_byte(0x01),
                rssi: -45,
                signal_type: ScanSignalType::Wifi,
                device_type: DetectedDeviceType::MacBook,
                vendor_hash: None,
                name_hash: None,
                frequency: Some(5180),
            };

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(vec![device.clone()])
            ));

            let tracked = DeviceScanner::tracked_devices(device.mac_hash).unwrap();
            assert_eq!(tracked.signal_type, ScanSignalType::Wifi);
        });
    }

    #[test]
    fn test_bluetooth_signal_processing() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let device = ScannedDevice {
                mac_hash: H256::repeat_byte(0x02),
                rssi: -65,
                signal_type: ScanSignalType::Bluetooth,
                device_type: DetectedDeviceType::AirPods,
                vendor_hash: None,
                name_hash: None,
                frequency: None,
            };

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(vec![device.clone()])
            ));

            let tracked = DeviceScanner::tracked_devices(device.mac_hash).unwrap();
            assert_eq!(tracked.signal_type, ScanSignalType::Bluetooth);
        });
    }

    #[test]
    fn test_ble_signal_processing() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let device = ScannedDevice {
                mac_hash: H256::repeat_byte(0x03),
                rssi: -80,
                signal_type: ScanSignalType::Ble,
                device_type: DetectedDeviceType::AppleWatch,
                vendor_hash: None,
                name_hash: None,
                frequency: None,
            };

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(vec![device.clone()])
            ));

            let tracked = DeviceScanner::tracked_devices(device.mac_hash).unwrap();
            assert_eq!(tracked.signal_type, ScanSignalType::Ble);
        });
    }

    #[test]
    fn test_on_initialize_resets_scan_flag() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let device = make_device(0x01, -50, DetectedDeviceType::IPhone);
            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(vec![device])
            ));

            assert!(crate::pallet::ScanDataReceived::<Test>::get());

            DeviceScanner::on_initialize(2);

            assert!(!crate::pallet::ScanDataReceived::<Test>::get());
        });
    }

    #[test]
    fn test_device_type_detection_all_types() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let types = vec![
                DetectedDeviceType::Unknown,
                DetectedDeviceType::IPhone,
                DetectedDeviceType::Android,
                DetectedDeviceType::MacBook,
                DetectedDeviceType::WindowsPC,
                DetectedDeviceType::LinuxPC,
                DetectedDeviceType::IPad,
                DetectedDeviceType::AppleWatch,
                DetectedDeviceType::AirPods,
                DetectedDeviceType::SmartTV,
                DetectedDeviceType::IoTDevice,
                DetectedDeviceType::NetworkDevice,
                DetectedDeviceType::Printer,
                DetectedDeviceType::GameConsole,
            ];

            let devices: Vec<ScannedDevice> = types
                .iter()
                .enumerate()
                .map(|(i, t)| make_device(i as u8, -50, *t))
                .collect();

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(devices)
            ));

            assert_eq!(DeviceScanner::device_count(), types.len() as u64);

            for device_type in types {
                assert_eq!(DeviceScanner::device_type_count(device_type), 1);
            }
        });
    }

    #[test]
    fn test_get_active_devices() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let devices = vec![
                make_device(0x01, -50, DetectedDeviceType::IPhone),
                make_device(0x02, -60, DetectedDeviceType::Android),
            ];

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(devices)
            ));

            let active = DeviceScanner::get_active_devices();
            assert_eq!(active.len(), 2);
        });
    }

    #[test]
    fn test_rssi_negative_values() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let device = make_device(0x01, -100, DetectedDeviceType::IPhone);
            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(vec![device.clone()])
            ));

            let tracked = DeviceScanner::tracked_devices(device.mac_hash).unwrap();
            assert_eq!(tracked.last_rssi, -100);
        });
    }

    #[test]
    fn test_rssi_boundary_values() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let device_min = make_device(0x01, i8::MIN, DetectedDeviceType::IPhone);
            let device_max = make_device(0x02, i8::MAX, DetectedDeviceType::Android);

            assert_ok!(DeviceScanner::set_scan_data(
                RuntimeOrigin::none(),
                make_scan_data(vec![device_min.clone(), device_max.clone()])
            ));

            let tracked_min = DeviceScanner::tracked_devices(device_min.mac_hash).unwrap();
            let tracked_max = DeviceScanner::tracked_devices(device_max.mac_hash).unwrap();

            assert_eq!(tracked_min.last_rssi, i8::MIN);
            assert_eq!(tracked_max.last_rssi, i8::MAX);
        });
    }

    #[test]
    fn test_negative_position_coordinates() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);

            let device = make_device(0x01, -50, DetectedDeviceType::IPhone);
            let mut data = make_scan_data(vec![device.clone()]);
            data.reporter_position = Position { x: -100, y: -200, z: -50 };

            assert_ok!(DeviceScanner::set_scan_data(RuntimeOrigin::none(), data));

            let tracked = DeviceScanner::tracked_devices(device.mac_hash).unwrap();
            assert_eq!(tracked.last_position.x, -100);
            assert_eq!(tracked.last_position.y, -200);
            assert_eq!(tracked.last_position.z, -50);
        });
    }
}
