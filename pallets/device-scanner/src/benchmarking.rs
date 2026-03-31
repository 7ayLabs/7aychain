//! Benchmarking setup for pallet-device-scanner
//!
//! SPDX-License-Identifier: BUSL-1.1

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_core::H256;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn set_scan_data(n: Linear<1, 100>) {
        let devices: Vec<ScannedDevice> = (0..n)
            .map(|i| {
                let mut bytes = [0u8; 32];
                bytes[0] = (i & 0xFF) as u8;
                bytes[1] = ((i >> 8) & 0xFF) as u8;
                ScannedDevice {
                    mac_hash: H256::from(bytes),
                    rssi: -65,
                    signal_type: ScanSignalType::Wifi,
                    device_type: DetectedDeviceType::Unknown,
                    vendor_hash: None,
                    name_hash: None,
                    frequency: Some(2400),
                }
            })
            .collect();
        let data = DeviceScanInherentData {
            devices,
            reporter_position: Position {
                x: 40_000,
                y: -74_000,
                z: 0,
            },
            scan_timestamp: 1_000_000,
        };

        #[extrinsic_call]
        _(RawOrigin::None, data);
    }

    impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test);
}
