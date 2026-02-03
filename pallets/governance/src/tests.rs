#![allow(clippy::disallowed_macros)]

use crate::{
    self as pallet_governance, CapabilityId, CapabilityStatus, Error, Event, Permissions,
    ResourceId,
};
use frame_support::{
    assert_noop, assert_ok, derive_impl, parameter_types,
    traits::{ConstU32, Hooks},
};
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
        Governance: pallet_governance,
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
    pub const MaxCapabilitiesPerActor: u32 = 20;
    pub const MaxDelegationDepth: u32 = 5;
    pub const DefaultCapabilityDuration: u64 = 1000;
    pub const MaxCapabilitiesPerResource: u32 = 50;
}

impl pallet_governance::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxCapabilitiesPerActor = MaxCapabilitiesPerActor;
    type MaxDelegationDepth = MaxDelegationDepth;
    type DefaultCapabilityDuration = DefaultCapabilityDuration;
    type MaxCapabilitiesPerResource = MaxCapabilitiesPerResource;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_governance::GenesisConfig::<Test> {
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

fn test_resource(id: u8) -> ResourceId {
    let mut bytes = [0u8; 32];
    bytes[0] = id;
    ResourceId::from_bytes(bytes)
}

#[test]
fn grant_capability_success() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(2);
        let resource = test_resource(1);
        let permissions = Permissions::READ.union(Permissions::WRITE);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(1),
            grantee,
            resource,
            permissions,
            Some(100),
            true
        ));

        let capability_id = CapabilityId::new(0);
        let capability = Governance::capabilities(capability_id).expect("capability should exist");

        assert_eq!(capability.grantor, 1);
        assert_eq!(capability.grantee, grantee);
        assert_eq!(capability.resource, resource);
        assert_eq!(capability.permissions, permissions);
        assert_eq!(capability.status, CapabilityStatus::Active);
        assert_eq!(capability.expires_at, Some(100));
        assert!(capability.delegatable);
    });
}

#[test]
fn grant_capability_invalid_permissions() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(2);
        let resource = test_resource(1);

        assert_noop!(
            Governance::grant_capability(
                RuntimeOrigin::signed(1),
                grantee,
                resource,
                Permissions::NONE,
                None,
                false
            ),
            Error::<Test>::InvalidPermissions
        );
    });
}

#[test]
fn revoke_capability_success() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(2);
        let resource = test_resource(1);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(1),
            grantee,
            resource,
            Permissions::READ,
            None,
            false
        ));

        let capability_id = CapabilityId::new(0);

        assert_ok!(Governance::revoke_capability(
            RuntimeOrigin::signed(1),
            capability_id
        ));

        let capability = Governance::capabilities(capability_id).expect("capability should exist");
        assert_eq!(capability.status, CapabilityStatus::Revoked);
    });
}

#[test]
fn revoke_capability_not_authorized() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(2);
        let resource = test_resource(1);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(1),
            grantee,
            resource,
            Permissions::READ,
            None,
            false
        ));

        let capability_id = CapabilityId::new(0);

        assert_noop!(
            Governance::revoke_capability(RuntimeOrigin::signed(3), capability_id),
            Error::<Test>::NotAuthorized
        );
    });
}

#[test]
fn delegate_capability_success() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(1);
        let delegatee = account_to_actor(2);
        let resource = test_resource(1);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(99),
            grantee,
            resource,
            Permissions::READ.union(Permissions::WRITE),
            None,
            true
        ));

        let capability_id = CapabilityId::new(0);

        assert_ok!(Governance::delegate_capability(
            RuntimeOrigin::signed(1),
            capability_id,
            delegatee,
            Permissions::READ,
            Some(500)
        ));

        let delegated_capability_id = CapabilityId::new(1);
        let delegated =
            Governance::capabilities(delegated_capability_id).expect("delegated cap should exist");

        assert_eq!(delegated.grantee, delegatee);
        assert_eq!(delegated.permissions, Permissions::READ);
        assert_eq!(delegated.parent_capability, Some(capability_id));
    });
}

#[test]
fn delegate_capability_not_delegatable() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(1);
        let delegatee = account_to_actor(2);
        let resource = test_resource(1);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(99),
            grantee,
            resource,
            Permissions::READ,
            None,
            false
        ));

        let capability_id = CapabilityId::new(0);

        assert_noop!(
            Governance::delegate_capability(
                RuntimeOrigin::signed(1),
                capability_id,
                delegatee,
                Permissions::READ,
                None
            ),
            Error::<Test>::CapabilityNotDelegatable
        );
    });
}

#[test]
fn delegate_capability_insufficient_permissions() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(1);
        let delegatee = account_to_actor(2);
        let resource = test_resource(1);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(99),
            grantee,
            resource,
            Permissions::READ,
            None,
            true
        ));

        let capability_id = CapabilityId::new(0);

        assert_noop!(
            Governance::delegate_capability(
                RuntimeOrigin::signed(1),
                capability_id,
                delegatee,
                Permissions::WRITE,
                None
            ),
            Error::<Test>::InsufficientPermissions
        );
    });
}

#[test]
fn delegate_capability_max_depth_reached() {
    new_test_ext().execute_with(|| {
        let resource = test_resource(1);

        let actor0 = account_to_actor(0);
        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(99),
            actor0,
            resource,
            Permissions::READ,
            None,
            true
        ));

        for i in 0..5 {
            let delegatee = account_to_actor(i + 1);
            assert_ok!(Governance::delegate_capability(
                RuntimeOrigin::signed(i),
                CapabilityId::new(i),
                delegatee,
                Permissions::READ,
                None
            ));
        }

        let final_delegatee = account_to_actor(6);
        assert_noop!(
            Governance::delegate_capability(
                RuntimeOrigin::signed(5),
                CapabilityId::new(5),
                final_delegatee,
                Permissions::READ,
                None
            ),
            Error::<Test>::MaxDelegationDepthReached
        );
    });
}

#[test]
fn update_capability_success() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(2);
        let resource = test_resource(1);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(1),
            grantee,
            resource,
            Permissions::READ,
            None,
            false
        ));

        let capability_id = CapabilityId::new(0);

        assert_ok!(Governance::update_capability(
            RuntimeOrigin::signed(1),
            capability_id,
            Permissions::READ.union(Permissions::WRITE)
        ));

        let capability = Governance::capabilities(capability_id).expect("capability should exist");
        assert_eq!(
            capability.permissions,
            Permissions::READ.union(Permissions::WRITE)
        );
    });
}

#[test]
fn has_permission_helper() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(2);
        let resource = test_resource(1);

        assert!(!Governance::has_permission(actor, resource, Permissions::READ));

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(1),
            actor,
            resource,
            Permissions::READ.union(Permissions::WRITE),
            None,
            false
        ));

        assert!(Governance::has_permission(actor, resource, Permissions::READ));
        assert!(Governance::has_permission(actor, resource, Permissions::WRITE));
        assert!(!Governance::has_permission(
            actor,
            resource,
            Permissions::EXECUTE
        ));
    });
}

#[test]
fn capability_expires() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(2);
        let resource = test_resource(1);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(1),
            grantee,
            resource,
            Permissions::READ,
            Some(10),
            false
        ));

        assert!(Governance::has_permission(grantee, resource, Permissions::READ));

        System::set_block_number(10);
        Governance::on_initialize(10);

        let capability_id = CapabilityId::new(0);
        let capability = Governance::capabilities(capability_id).expect("capability should exist");
        assert_eq!(capability.status, CapabilityStatus::Expired);
    });
}

#[test]
fn revoking_parent_revokes_delegated() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(1);
        let delegatee = account_to_actor(2);
        let resource = test_resource(1);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(99),
            grantee,
            resource,
            Permissions::READ,
            None,
            true
        ));

        let parent_id = CapabilityId::new(0);

        assert_ok!(Governance::delegate_capability(
            RuntimeOrigin::signed(1),
            parent_id,
            delegatee,
            Permissions::READ,
            None
        ));

        let delegated_id = CapabilityId::new(1);

        assert_ok!(Governance::revoke_capability(
            RuntimeOrigin::signed(99),
            parent_id
        ));

        let delegated = Governance::capabilities(delegated_id).expect("delegated cap should exist");
        assert_eq!(delegated.status, CapabilityStatus::Revoked);
    });
}

#[test]
fn get_actor_capabilities_helper() {
    new_test_ext().execute_with(|| {
        let actor = account_to_actor(2);
        let resource1 = test_resource(1);
        let resource2 = test_resource(2);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(1),
            actor,
            resource1,
            Permissions::READ,
            None,
            false
        ));

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(1),
            actor,
            resource2,
            Permissions::WRITE,
            None,
            false
        ));

        let capabilities = Governance::get_actor_capabilities(actor);
        assert_eq!(capabilities.len(), 2);
    });
}

#[test]
fn get_delegation_chain_helper() {
    new_test_ext().execute_with(|| {
        let resource = test_resource(1);

        let actor0 = account_to_actor(0);
        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(99),
            actor0,
            resource,
            Permissions::READ,
            None,
            true
        ));

        for i in 0..3 {
            let delegatee = account_to_actor(i + 1);
            assert_ok!(Governance::delegate_capability(
                RuntimeOrigin::signed(i),
                CapabilityId::new(i),
                delegatee,
                Permissions::READ,
                None
            ));
        }

        let chain = Governance::get_delegation_chain(CapabilityId::new(3));
        assert_eq!(chain.len(), 4);
        assert_eq!(chain[0], CapabilityId::new(0));
        assert_eq!(chain[3], CapabilityId::new(3));
    });
}

#[test]
fn is_capability_active_helper() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(2);
        let resource = test_resource(1);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(1),
            grantee,
            resource,
            Permissions::READ,
            Some(50),
            false
        ));

        let capability_id = CapabilityId::new(0);

        assert!(Governance::is_capability_active(capability_id));

        System::set_block_number(50);

        assert!(!Governance::is_capability_active(capability_id));
    });
}

#[test]
fn permissions_operations() {
    let read = Permissions::READ;
    let write = Permissions::WRITE;
    let read_write = read.union(write);

    assert!(read_write.contains(read));
    assert!(read_write.contains(write));
    assert!(!read_write.contains(Permissions::EXECUTE));

    let intersection = read_write.intersection(read);
    assert_eq!(intersection, read);

    assert!(!read.is_empty());
    assert!(Permissions::NONE.is_empty());
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(2);
        let resource = test_resource(1);
        let permissions = Permissions::READ;

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(1),
            grantee,
            resource,
            permissions,
            None,
            false
        ));

        System::assert_has_event(RuntimeEvent::Governance(Event::CapabilityGranted {
            capability_id: CapabilityId::new(0),
            grantor: 1,
            grantee,
            resource,
            permissions,
        }));
    });
}

#[test]
fn self_delegation_prevented() {
    new_test_ext().execute_with(|| {
        let grantee = account_to_actor(1);
        let resource = test_resource(1);

        assert_ok!(Governance::grant_capability(
            RuntimeOrigin::signed(99),
            grantee,
            resource,
            Permissions::READ,
            None,
            true
        ));

        let capability_id = CapabilityId::new(0);

        assert_noop!(
            Governance::delegate_capability(
                RuntimeOrigin::signed(1),
                capability_id,
                grantee,
                Permissions::READ,
                None
            ),
            Error::<Test>::SelfDelegation
        );
    });
}

#[test]
fn genesis_initializes_capability_count() {
    new_test_ext().execute_with(|| {
        assert_eq!(Governance::capability_count(), 0);
    });
}
