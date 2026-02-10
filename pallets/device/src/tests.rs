#![allow(clippy::disallowed_macros)]

use crate::{
    self as pallet_device, AttestationType, DeviceId, DeviceStatus, DeviceType, Error, Event,
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
        Device: pallet_device,
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
    pub const MaxDevicesPerActor: u32 = 10;
    pub const AttestationValidityBlocks: u64 = 1000;
    pub const InitialTrustScore: u8 = 50;
}

impl pallet_device::Config for Test {
    type WeightInfo = ();
    type MaxDevicesPerActor = MaxDevicesPerActor;
    type AttestationValidityBlocks = AttestationValidityBlocks;
    type InitialTrustScore = InitialTrustScore;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_device::GenesisConfig::<Test> {
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
fn register_device_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let public_key = H256([1u8; 32]);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            public_key,
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);
        let device = Device::devices(device_id).expect("device should exist");

        assert_eq!(device.owner, owner);
        assert_eq!(device.device_type, DeviceType::Mobile);
        assert_eq!(device.status, DeviceStatus::Pending);
        assert_eq!(device.trust_score, 50);
    });
}

#[test]
fn public_key_uniqueness_enforced() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let public_key = H256([1u8; 32]);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            public_key,
            AttestationType::SelfSigned
        ));

        assert_noop!(
            Device::register_device(
                RuntimeOrigin::signed(1),
                owner,
                DeviceType::Desktop,
                public_key,
                AttestationType::SelfSigned
            ),
            Error::<Test>::PublicKeyAlreadyUsed
        );
    });
}

#[test]
fn max_devices_enforced() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        for i in 0..10 {
            assert_ok!(Device::register_device(
                RuntimeOrigin::signed(1),
                owner,
                DeviceType::Mobile,
                H256([i as u8; 32]),
                AttestationType::SelfSigned
            ));
        }

        assert_noop!(
            Device::register_device(
                RuntimeOrigin::signed(1),
                owner,
                DeviceType::Mobile,
                H256([100u8; 32]),
                AttestationType::SelfSigned
            ),
            Error::<Test>::MaxDevicesReached
        );
    });
}

#[test]
fn activate_device_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);
        assert_ok!(Device::activate_device(RuntimeOrigin::signed(1), device_id));

        let device = Device::devices(device_id).expect("device should exist");
        assert_eq!(device.status, DeviceStatus::Active);
        assert_eq!(Device::get_total_active_devices(), 1);
    });
}

#[test]
fn cannot_activate_already_active() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);
        assert_ok!(Device::activate_device(RuntimeOrigin::signed(1), device_id));

        assert_noop!(
            Device::activate_device(RuntimeOrigin::signed(1), device_id),
            Error::<Test>::DeviceAlreadyActive
        );
    });
}

#[test]
fn suspend_device_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);
        assert_ok!(Device::activate_device(RuntimeOrigin::signed(1), device_id));
        assert_ok!(Device::suspend_device(
            RuntimeOrigin::signed(1),
            device_id,
            H256([0u8; 32])
        ));

        let device = Device::devices(device_id).expect("device should exist");
        assert_eq!(device.status, DeviceStatus::Suspended);
        assert_eq!(Device::get_total_active_devices(), 0);
    });
}

#[test]
fn revoke_device_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);
        assert_ok!(Device::revoke_device(RuntimeOrigin::signed(1), device_id));

        let device = Device::devices(device_id).expect("device should exist");
        assert_eq!(device.status, DeviceStatus::Revoked);
    });
}

#[test]
fn mark_compromised_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);
        assert_ok!(Device::activate_device(RuntimeOrigin::signed(1), device_id));
        assert_ok!(Device::mark_compromised(RuntimeOrigin::root(), device_id));

        let device = Device::devices(device_id).expect("device should exist");
        assert_eq!(device.status, DeviceStatus::Compromised);
        assert_eq!(Device::get_total_active_devices(), 0);
    });
}

#[test]
fn submit_attestation_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);
        let attestation_hash = H256([2u8; 32]);

        assert_ok!(Device::submit_attestation(
            RuntimeOrigin::signed(1),
            device_id,
            attestation_hash,
            None
        ));

        let attestation = Device::attestations(device_id).expect("attestation should exist");
        assert_eq!(attestation.attestation_hash, attestation_hash);
        assert!(attestation.valid_until.is_some());
    });
}

#[test]
fn update_trust_score_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);

        assert_ok!(Device::update_trust_score(
            RuntimeOrigin::root(),
            device_id,
            80
        ));

        assert_eq!(Device::get_device_trust_score(device_id), 80);
    });
}

#[test]
fn invalid_trust_score_rejected() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        assert_noop!(
            Device::update_trust_score(RuntimeOrigin::root(), DeviceId::new(0), 101),
            Error::<Test>::InvalidTrustScore
        );
    });
}

#[test]
fn record_activity_success() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);
        assert_ok!(Device::activate_device(RuntimeOrigin::signed(1), device_id));

        System::set_block_number(100);
        assert_ok!(Device::record_activity(RuntimeOrigin::signed(1), device_id));

        let device = Device::devices(device_id).expect("device should exist");
        assert_eq!(device.last_active, 100);
    });
}

#[test]
fn reactivate_suspended_device() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);
        assert_ok!(Device::activate_device(RuntimeOrigin::signed(1), device_id));
        assert_ok!(Device::suspend_device(
            RuntimeOrigin::signed(1),
            device_id,
            H256([0u8; 32])
        ));

        assert_ok!(Device::reactivate_device(
            RuntimeOrigin::signed(1),
            device_id
        ));

        let device = Device::devices(device_id).expect("device should exist");
        assert_eq!(device.status, DeviceStatus::Active);
        assert_eq!(Device::get_total_active_devices(), 1);
    });
}

#[test]
fn cannot_reactivate_revoked_device() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);
        assert_ok!(Device::revoke_device(RuntimeOrigin::signed(1), device_id));

        assert_noop!(
            Device::reactivate_device(RuntimeOrigin::signed(1), device_id),
            Error::<Test>::CannotReactivateRevokedDevice
        );
    });
}

#[test]
fn get_actor_devices_helper() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        for i in 0..3 {
            assert_ok!(Device::register_device(
                RuntimeOrigin::signed(1),
                owner,
                DeviceType::Mobile,
                H256([i as u8; 32]),
                AttestationType::SelfSigned
            ));
        }

        let devices = Device::get_actor_devices(owner);
        assert_eq!(devices.len(), 3);
    });
}

#[test]
fn get_active_devices_helper() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        for i in 0..3 {
            assert_ok!(Device::register_device(
                RuntimeOrigin::signed(1),
                owner,
                DeviceType::Mobile,
                H256([i as u8; 32]),
                AttestationType::SelfSigned
            ));
        }

        assert_ok!(Device::activate_device(
            RuntimeOrigin::signed(1),
            DeviceId::new(0)
        ));
        assert_ok!(Device::activate_device(
            RuntimeOrigin::signed(1),
            DeviceId::new(1)
        ));

        let active = Device::get_active_devices(owner);
        assert_eq!(active.len(), 2);
    });
}

#[test]
fn is_attestation_valid_helper() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device_id = DeviceId::new(0);

        assert_ok!(Device::submit_attestation(
            RuntimeOrigin::signed(1),
            device_id,
            H256([2u8; 32]),
            None
        ));

        assert!(Device::is_attestation_valid(device_id, 500));
        assert!(!Device::is_attestation_valid(device_id, 1002));
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Mobile,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        System::assert_has_event(RuntimeEvent::Device(Event::DeviceRegistered {
            device_id: DeviceId::new(0),
            owner,
            device_type: DeviceType::Mobile,
        }));
    });
}

#[test]
fn genesis_initializes_counts() {
    new_test_ext().execute_with(|| {
        assert_eq!(Device::device_count(), 0);
        assert_eq!(Device::get_total_active_devices(), 0);
    });
}

#[test]
fn register_desktop_device() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Desktop,
            H256([1u8; 32]),
            AttestationType::TrustedParty
        ));

        let device = Device::devices(DeviceId::new(0)).expect("device should exist");
        assert_eq!(device.device_type, DeviceType::Desktop);
        assert_eq!(device.attestation_type, AttestationType::TrustedParty);
    });
}

#[test]
fn register_server_device() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Server,
            H256([1u8; 32]),
            AttestationType::HardwareBacked
        ));

        let device = Device::devices(DeviceId::new(0)).expect("device should exist");
        assert_eq!(device.device_type, DeviceType::Server);
        assert_eq!(device.attestation_type, AttestationType::HardwareBacked);
    });
}

#[test]
fn register_iot_device() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::IoT,
            H256([1u8; 32]),
            AttestationType::Tpm
        ));

        let device = Device::devices(DeviceId::new(0)).expect("device should exist");
        assert_eq!(device.device_type, DeviceType::IoT);
        assert_eq!(device.attestation_type, AttestationType::Tpm);
    });
}

#[test]
fn register_hardware_device() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Hardware,
            H256([1u8; 32]),
            AttestationType::SecureEnclave
        ));

        let device = Device::devices(DeviceId::new(0)).expect("device should exist");
        assert_eq!(device.device_type, DeviceType::Hardware);
        assert_eq!(device.attestation_type, AttestationType::SecureEnclave);
    });
}

#[test]
fn register_virtual_device() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Virtual,
            H256([1u8; 32]),
            AttestationType::SelfSigned
        ));

        let device = Device::devices(DeviceId::new(0)).expect("device should exist");
        assert_eq!(device.device_type, DeviceType::Virtual);
    });
}

#[test]
fn register_all_device_types() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let device_types = [
            DeviceType::Mobile,
            DeviceType::Desktop,
            DeviceType::Server,
            DeviceType::IoT,
            DeviceType::Hardware,
            DeviceType::Virtual,
        ];

        for (i, device_type) in device_types.iter().enumerate() {
            assert_ok!(Device::register_device(
                RuntimeOrigin::signed(1),
                owner,
                *device_type,
                H256([i as u8; 32]),
                AttestationType::SelfSigned
            ));

            let device = Device::devices(DeviceId::new(i as u64)).expect("device should exist");
            assert_eq!(device.device_type, *device_type);
        }

        assert_eq!(Device::device_count(), 6);
    });
}

#[test]
fn all_attestation_types() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let attestation_types = [
            AttestationType::SelfSigned,
            AttestationType::TrustedParty,
            AttestationType::HardwareBacked,
            AttestationType::Tpm,
            AttestationType::SecureEnclave,
        ];

        for (i, attestation_type) in attestation_types.iter().enumerate() {
            assert_ok!(Device::register_device(
                RuntimeOrigin::signed(1),
                owner,
                DeviceType::Mobile,
                H256([i as u8; 32]),
                *attestation_type
            ));

            let device = Device::devices(DeviceId::new(i as u64)).expect("device should exist");
            assert_eq!(device.attestation_type, *attestation_type);
        }
    });
}

#[test]
fn device_lifecycle_server() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Server,
            H256([1u8; 32]),
            AttestationType::HardwareBacked
        ));

        let device_id = DeviceId::new(0);

        assert_ok!(Device::activate_device(RuntimeOrigin::signed(1), device_id));
        assert!(Device::is_device_active(device_id));

        assert_ok!(Device::suspend_device(
            RuntimeOrigin::signed(1),
            device_id,
            H256([0u8; 32])
        ));
        assert!(!Device::is_device_active(device_id));

        assert_ok!(Device::reactivate_device(RuntimeOrigin::signed(1), device_id));
        assert!(Device::is_device_active(device_id));

        assert_ok!(Device::revoke_device(RuntimeOrigin::signed(1), device_id));
        assert!(!Device::is_device_active(device_id));
    });
}

#[test]
fn device_lifecycle_iot() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::IoT,
            H256([1u8; 32]),
            AttestationType::Tpm
        ));

        let device_id = DeviceId::new(0);

        assert_ok!(Device::activate_device(RuntimeOrigin::signed(1), device_id));
        assert!(Device::is_device_active(device_id));

        assert_ok!(Device::mark_compromised(RuntimeOrigin::root(), device_id));

        let device = Device::devices(device_id).expect("device should exist");
        assert_eq!(device.status, DeviceStatus::Compromised);
    });
}

#[test]
fn attestation_with_attester() {
    new_test_ext().execute_with(|| {
        let owner = account_to_actor(1);
        let attester = account_to_actor(2);

        assert_ok!(Device::register_device(
            RuntimeOrigin::signed(1),
            owner,
            DeviceType::Hardware,
            H256([1u8; 32]),
            AttestationType::TrustedParty
        ));

        let device_id = DeviceId::new(0);

        assert_ok!(Device::submit_attestation(
            RuntimeOrigin::signed(1),
            device_id,
            H256([2u8; 32]),
            Some(attester)
        ));

        let attestation = Device::attestations(device_id).expect("attestation should exist");
        assert_eq!(attestation.attester, Some(attester));
    });
}
