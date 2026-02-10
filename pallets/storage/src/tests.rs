use crate::{self as pallet_storage, *};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, ConstU64},
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
        Storage: pallet_storage,
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
    type BlockHashCount = ConstU64<250>;
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
    pub const MaxDataSize: u32 = 1024;
    pub const MaxEntriesPerActor: u32 = 100;
    pub const MaxEntriesPerEpoch: u32 = 1000;
    pub const DefaultRetentionBlocks: u64 = 100;
}

impl pallet_storage::Config for Test {
    type WeightInfo = ();
    type MaxDataSize = MaxDataSize;
    type MaxEntriesPerActor = MaxEntriesPerActor;
    type MaxEntriesPerEpoch = MaxEntriesPerEpoch;
    type DefaultRetentionBlocks = DefaultRetentionBlocks;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("failed to build storage");
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn create_epoch() -> EpochId {
    EpochId::new(1)
}

fn create_key(seed: u8) -> DataKey {
    DataKey::new(H256([seed; 32]))
}

fn create_hash(seed: u8) -> H256 {
    H256([seed; 32])
}

fn account_to_actor(account: u64) -> ActorId {
    use parity_scale_codec::Encode;
    let encoded = account.encode();
    let hash = sp_core::blake2_256(&encoded);
    ActorId::from_raw(hash)
}

#[test]
fn store_data_success() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();
        let key = create_key(1);
        let data_hash = create_hash(2);

        assert_ok!(Storage::store_data(
            RuntimeOrigin::signed(1),
            epoch,
            key,
            data_hash,
            DataType::Presence,
            100,
            RetentionPolicy::EpochBound
        ));

        assert_eq!(Storage::total_entries(), 1);
        assert_eq!(Storage::total_active(), 1);
        assert_eq!(Storage::total_bytes(), 100);
    });
}

#[test]
fn cannot_store_duplicate_data() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();
        let key = create_key(1);
        let data_hash = create_hash(2);

        assert_ok!(Storage::store_data(
            RuntimeOrigin::signed(1),
            epoch,
            key,
            data_hash,
            DataType::Presence,
            100,
            RetentionPolicy::EpochBound
        ));

        assert_noop!(
            Storage::store_data(
                RuntimeOrigin::signed(1),
                epoch,
                key,
                data_hash,
                DataType::Presence,
                100,
                RetentionPolicy::EpochBound
            ),
            Error::<Test>::DataAlreadyExists
        );
    });
}

#[test]
fn data_too_large_rejected() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();
        let key = create_key(1);
        let data_hash = create_hash(2);

        assert_noop!(
            Storage::store_data(
                RuntimeOrigin::signed(1),
                epoch,
                key,
                data_hash,
                DataType::Presence,
                2000,
                RetentionPolicy::EpochBound
            ),
            Error::<Test>::DataTooLarge
        );
    });
}

#[test]
fn update_data_success() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();
        let key = create_key(1);
        let data_hash = create_hash(2);
        let new_hash = create_hash(3);

        assert_ok!(Storage::store_data(
            RuntimeOrigin::signed(1),
            epoch,
            key,
            data_hash,
            DataType::Presence,
            100,
            RetentionPolicy::EpochBound
        ));

        assert_ok!(Storage::update_data(
            RuntimeOrigin::signed(1),
            epoch,
            key,
            new_hash,
            150
        ));

        assert_eq!(Storage::total_bytes(), 150);
    });
}

#[test]
fn update_nonexistent_data_fails() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();
        let key = create_key(1);
        let data_hash = create_hash(2);

        assert_noop!(
            Storage::update_data(RuntimeOrigin::signed(1), epoch, key, data_hash, 100),
            Error::<Test>::DataNotFound
        );
    });
}

#[test]
fn delete_data_success() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();
        let key = create_key(1);
        let data_hash = create_hash(2);

        assert_ok!(Storage::store_data(
            RuntimeOrigin::signed(1),
            epoch,
            key,
            data_hash,
            DataType::Presence,
            100,
            RetentionPolicy::EpochBound
        ));

        assert_ok!(Storage::delete_data(RuntimeOrigin::signed(1), epoch, key));

        assert_eq!(Storage::total_active(), 0);
        assert_eq!(Storage::total_bytes(), 0);
    });
}

#[test]
fn delete_nonexistent_data_fails() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();
        let key = create_key(1);

        assert_noop!(
            Storage::delete_data(RuntimeOrigin::signed(1), epoch, key),
            Error::<Test>::DataNotFound
        );
    });
}

#[test]
fn set_quota_success() {
    new_test_ext().execute_with(|| {
        let mut bytes = [0u8; 32];
        bytes[0] = 1;
        let actor = ActorId::from_raw(bytes);

        assert_ok!(Storage::set_quota(RuntimeOrigin::root(), actor, 50, 5000));

        let quota = Storage::storage_quotas(actor).expect("quota should exist");
        assert_eq!(quota.max_entries, 50);
        assert_eq!(quota.max_bytes, 5000);
    });
}

#[test]
fn finalize_epoch_success() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();
        let key = create_key(1);
        let data_hash = create_hash(2);

        assert_ok!(Storage::store_data(
            RuntimeOrigin::signed(1),
            epoch,
            key,
            data_hash,
            DataType::Presence,
            100,
            RetentionPolicy::EpochBound
        ));

        assert_ok!(Storage::finalize_epoch(RuntimeOrigin::root(), epoch));

        assert!(Storage::is_epoch_finalized(epoch));
    });
}

#[test]
fn different_data_types() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();

        let types = [
            DataType::Presence,
            DataType::Commitment,
            DataType::Proof,
            DataType::Metadata,
            DataType::Temporary,
        ];

        for (i, data_type) in types.iter().enumerate() {
            let key = create_key((i + 1) as u8);
            let data_hash = create_hash((i + 10) as u8);

            assert_ok!(Storage::store_data(
                RuntimeOrigin::signed(1),
                epoch,
                key,
                data_hash,
                *data_type,
                100,
                RetentionPolicy::EpochBound
            ));
        }

        assert_eq!(Storage::total_entries(), 5);
    });
}

#[test]
fn different_retention_policies() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();

        let policies = [
            RetentionPolicy::EpochBound,
            RetentionPolicy::TimeBound,
            RetentionPolicy::Persistent,
            RetentionPolicy::OneTime,
        ];

        for (i, policy) in policies.iter().enumerate() {
            let key = create_key((i + 1) as u8);
            let data_hash = create_hash((i + 10) as u8);

            assert_ok!(Storage::store_data(
                RuntimeOrigin::signed(1),
                epoch,
                key,
                data_hash,
                DataType::Presence,
                100,
                *policy
            ));
        }

        assert_eq!(Storage::total_entries(), 4);
    });
}

#[test]
fn entry_retrieval() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();
        let key = create_key(1);
        let data_hash = create_hash(2);

        assert_ok!(Storage::store_data(
            RuntimeOrigin::signed(1),
            epoch,
            key,
            data_hash,
            DataType::Commitment,
            100,
            RetentionPolicy::Persistent
        ));

        let actor = account_to_actor(1);

        let entry = Storage::get_entry(epoch, actor, key).expect("entry should exist");
        assert_eq!(entry.data_hash, data_hash);
        assert_eq!(entry.data_type, DataType::Commitment);
        assert_eq!(entry.retention, RetentionPolicy::Persistent);
    });
}

#[test]
fn actor_entry_count() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();

        for i in 1..=5 {
            let key = create_key(i);
            let data_hash = create_hash(i + 10);

            assert_ok!(Storage::store_data(
                RuntimeOrigin::signed(1),
                epoch,
                key,
                data_hash,
                DataType::Presence,
                100,
                RetentionPolicy::EpochBound
            ));
        }

        let actor = account_to_actor(1);

        assert_eq!(Storage::get_actor_entry_count(actor), 5);
    });
}

#[test]
fn epoch_entry_count() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();

        for i in 1..=3 {
            let key = create_key(i);
            let data_hash = create_hash(i + 10);

            assert_ok!(Storage::store_data(
                RuntimeOrigin::signed(i as u64),
                epoch,
                key,
                data_hash,
                DataType::Presence,
                100,
                RetentionPolicy::EpochBound
            ));
        }

        assert_eq!(Storage::get_epoch_entry_count(epoch), 3);
    });
}

#[test]
fn multiple_actors_same_epoch() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();
        let key = create_key(1);

        for i in 1..=3 {
            assert_ok!(Storage::store_data(
                RuntimeOrigin::signed(i),
                epoch,
                key,
                create_hash(i as u8),
                DataType::Presence,
                100,
                RetentionPolicy::EpochBound
            ));
        }

        assert_eq!(Storage::total_entries(), 3);
    });
}

#[test]
fn same_actor_multiple_epochs() {
    new_test_ext().execute_with(|| {
        let key = create_key(1);

        for i in 1..=3 {
            let epoch = EpochId::new(i as u64);
            assert_ok!(Storage::store_data(
                RuntimeOrigin::signed(1),
                epoch,
                key,
                create_hash(i as u8),
                DataType::Presence,
                100,
                RetentionPolicy::EpochBound
            ));
        }

        assert_eq!(Storage::total_entries(), 3);
    });
}

#[test]
fn data_key_from_bytes() {
    let key1 = DataKey::from_bytes(b"test_key");
    let key2 = DataKey::from_bytes(b"test_key");
    let key3 = DataKey::from_bytes(b"other_key");

    assert_eq!(key1, key2);
    assert_ne!(key1, key3);
}

#[test]
fn genesis_initializes_correctly() {
    new_test_ext().execute_with(|| {
        assert_eq!(Storage::entry_count(), 0);
        assert_eq!(Storage::active_entries(), 0);
        assert_eq!(Storage::total_storage_bytes(), 0);
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        System::reset_events();

        let epoch = create_epoch();
        let key = create_key(1);
        let data_hash = create_hash(2);

        assert_ok!(Storage::store_data(
            RuntimeOrigin::signed(1),
            epoch,
            key,
            data_hash,
            DataType::Presence,
            100,
            RetentionPolicy::EpochBound
        ));

        let events = System::events();
        assert!(events.iter().any(|e| matches!(
            &e.event,
            RuntimeEvent::Storage(Event::DataStored { .. })
        )));
    });
}

#[test]
fn update_decreases_size() {
    new_test_ext().execute_with(|| {
        let epoch = create_epoch();
        let key = create_key(1);
        let data_hash = create_hash(2);

        assert_ok!(Storage::store_data(
            RuntimeOrigin::signed(1),
            epoch,
            key,
            data_hash,
            DataType::Presence,
            200,
            RetentionPolicy::EpochBound
        ));

        assert_eq!(Storage::total_bytes(), 200);

        assert_ok!(Storage::update_data(
            RuntimeOrigin::signed(1),
            epoch,
            key,
            create_hash(3),
            100
        ));

        assert_eq!(Storage::total_bytes(), 100);
    });
}
