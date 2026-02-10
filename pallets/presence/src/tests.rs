#![allow(clippy::disallowed_macros)]

use crate::{self as pallet_presence, Error, Event};
use frame_support::{assert_noop, assert_ok, derive_impl, parameter_types, traits::ConstU32};
use frame_system as system;
use seveny_primitives::{
    types::{ActorId, EpochId, PresenceState, ValidatorId},
    CryptoCommitment as Commitment,
};
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, Hash, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test {
        System: frame_system,
        Presence: pallet_presence,
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
    pub const MaxVotesPerPresence: u32 = 100;
    pub const DefaultQuorumThreshold: u32 = 3;
    pub const DefaultQuorumTotal: u32 = 5;
    pub const CommitRevealDelay: u64 = 10;
    pub const RevealWindow: u64 = 20;
}

impl pallet_presence::Config for Test {
    type WeightInfo = ();
    type MaxVotesPerPresence = MaxVotesPerPresence;
    type DefaultQuorumThreshold = DefaultQuorumThreshold;
    type DefaultQuorumTotal = DefaultQuorumTotal;
    type CommitRevealDelay = CommitRevealDelay;
    type RevealWindow = RevealWindow;
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::<Test>::default()
        .build_storage()
        .expect("storage build failed");

    pallet_presence::GenesisConfig::<Test> {
        quorum_threshold: 3,
        quorum_total: 5,
        initial_validators: vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32], [5u8; 32]],
        initial_epoch: 1,
        _phantom: Default::default(),
    }
    .assimilate_storage(&mut t)
    .expect("genesis build failed");

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}

fn account_to_actor(account: u64) -> ActorId {
    let hash = BlakeTwo256::hash_of(&account);
    ActorId::from(H256(hash.0))
}

fn account_to_validator(account: u64) -> ValidatorId {
    let hash = BlakeTwo256::hash_of(&account);
    ValidatorId::from(H256(hash.0))
}

fn setup_validator(account: u64) {
    let validator = account_to_validator(account);
    pallet_presence::ActiveValidators::<Test>::insert(validator, true);
}

#[test]
fn invariant_inv1_uniqueness_no_duplicate_presence() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));

        assert_noop!(
            Presence::declare_presence(RuntimeOrigin::signed(1), epoch),
            Error::<Test>::DuplicatePresence
        );
    });
}

#[test]
fn invariant_inv2_immutability_finalized_cannot_change() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));

        setup_validator(10);
        setup_validator(11);
        setup_validator(12);

        let actor = account_to_actor(1);

        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(10),
            actor,
            epoch,
            true
        ));
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(11),
            actor,
            epoch,
            true
        ));
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(12),
            actor,
            epoch,
            true
        ));

        assert_ok!(Presence::finalize_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch
        ));

        let record = Presence::presences(epoch, actor).expect("presence should exist");
        assert_eq!(record.state, PresenceState::Finalized);

        assert_noop!(
            Presence::slash_presence(RuntimeOrigin::root(), actor, epoch),
            Error::<Test>::PresenceImmutable
        );
    });
}

#[test]
fn invariant_inv7_monotonic_forward_only_transitions() {
    new_test_ext().execute_with(|| {
        assert!(PresenceState::None.can_transition_to(&PresenceState::Declared));
        assert!(PresenceState::Declared.can_transition_to(&PresenceState::Validated));
        assert!(PresenceState::Validated.can_transition_to(&PresenceState::Finalized));

        assert!(!PresenceState::Declared.can_transition_to(&PresenceState::None));
        assert!(!PresenceState::Validated.can_transition_to(&PresenceState::Declared));
        assert!(!PresenceState::Finalized.can_transition_to(&PresenceState::Validated));
    });
}

#[test]
fn invariant_inv8_terminal_states_cannot_transition() {
    new_test_ext().execute_with(|| {
        assert!(PresenceState::Finalized.is_terminal());
        assert!(PresenceState::Slashed.is_terminal());

        assert!(!PresenceState::Finalized.can_transition_to(&PresenceState::Slashed));
        assert!(!PresenceState::Slashed.can_transition_to(&PresenceState::Finalized));
    });
}

#[test]
fn invariant_inv9_epoch_expiry_cannot_declare_after_closed() {
    new_test_ext().execute_with(|| {
        let inactive_epoch = EpochId::new(999);

        assert_noop!(
            Presence::declare_presence(RuntimeOrigin::signed(1), inactive_epoch),
            Error::<Test>::EpochNotActive
        );
    });
}

#[test]
fn invariant_inv10_quorum_threshold_required() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));

        setup_validator(10);
        setup_validator(11);

        let actor = account_to_actor(1);

        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(10),
            actor,
            epoch,
            true
        ));
        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(11),
            actor,
            epoch,
            true
        ));

        let record = Presence::presences(epoch, actor).expect("presence should exist");
        assert_eq!(record.state, PresenceState::Declared);
        assert_eq!(record.vote_count, 2);

        assert_noop!(
            Presence::finalize_presence(RuntimeOrigin::signed(1), actor, epoch),
            Error::<Test>::PresenceNotValidated
        );
    });
}

#[test]
fn invariant_inv11_vote_uniqueness_one_vote_per_validator() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));

        setup_validator(10);
        let actor = account_to_actor(1);

        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(10),
            actor,
            epoch,
            true
        ));

        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(10), actor, epoch, true),
            Error::<Test>::DuplicateVote
        );

        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(10), actor, epoch, false),
            Error::<Test>::DuplicateVote
        );
    });
}

#[test]
fn invariant_inv12_vote_authorization_only_validators() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));

        let actor = account_to_actor(1);

        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(999), actor, epoch, true),
            Error::<Test>::ValidatorNotActive
        );
    });
}

#[test]
fn invariant_inv13_vote_timing_only_during_active_epoch() {
    new_test_ext().execute_with(|| {
        let inactive_epoch = EpochId::new(999);
        let actor = ActorId::from_raw([99u8; 32]);

        setup_validator(10);

        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(10), actor, inactive_epoch, true),
            Error::<Test>::EpochNotActive
        );
    });
}

#[test]
fn declare_presence_success() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));

        let actor = account_to_actor(1);
        let record = Presence::presences(epoch, actor).expect("presence should exist");

        assert_eq!(record.actor, actor);
        assert_eq!(record.epoch, epoch);
        assert_eq!(record.state, PresenceState::Declared);
        assert!(record.declared_at.is_some());
        assert!(record.validated_at.is_none());
        assert!(record.finalized_at.is_none());
        assert_eq!(record.vote_count, 0);

        assert_eq!(Presence::presence_count(epoch), 1);
    });
}

#[test]
fn declare_presence_with_commitment_success() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let randomness = [42u8; 32];
        let commitment = Commitment::new(&42u64, &randomness);

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment.clone()
        ));

        let actor = account_to_actor(1);
        let declaration = Presence::declarations(epoch, actor).expect("declaration should exist");

        assert_eq!(declaration.commitment, commitment);
    });
}

#[test]
fn full_presence_lifecycle() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));

        setup_validator(10);
        setup_validator(11);
        setup_validator(12);

        let actor = account_to_actor(1);

        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(10),
            actor,
            epoch,
            true
        ));

        let record = Presence::presences(epoch, actor).expect("presence should exist");
        assert_eq!(record.state, PresenceState::Declared);
        assert_eq!(record.vote_count, 1);

        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(11),
            actor,
            epoch,
            true
        ));

        let record = Presence::presences(epoch, actor).expect("presence should exist");
        assert_eq!(record.state, PresenceState::Declared);
        assert_eq!(record.vote_count, 2);

        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(12),
            actor,
            epoch,
            true
        ));

        let record = Presence::presences(epoch, actor).expect("presence should exist");
        assert_eq!(record.state, PresenceState::Validated);
        assert_eq!(record.vote_count, 3);
        assert!(record.validated_at.is_some());

        assert_ok!(Presence::finalize_presence(
            RuntimeOrigin::signed(1),
            actor,
            epoch
        ));

        let record = Presence::presences(epoch, actor).expect("presence should exist");
        assert_eq!(record.state, PresenceState::Finalized);
        assert!(record.finalized_at.is_some());
    });
}

#[test]
fn slash_presence_success() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));

        let actor = account_to_actor(1);

        assert_ok!(Presence::slash_presence(
            RuntimeOrigin::root(),
            actor,
            epoch
        ));

        let record = Presence::presences(epoch, actor).expect("presence should exist");
        assert_eq!(record.state, PresenceState::Slashed);
    });
}

#[test]
fn slashed_is_terminal() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));

        let actor = account_to_actor(1);

        assert_ok!(Presence::slash_presence(
            RuntimeOrigin::root(),
            actor,
            epoch
        ));

        setup_validator(10);
        assert_noop!(
            Presence::vote_presence(RuntimeOrigin::signed(10), actor, epoch, true),
            Error::<Test>::PresenceImmutable
        );
    });
}

#[test]
fn set_quorum_config_success() {
    new_test_ext().execute_with(|| {
        assert_ok!(Presence::set_quorum_config(RuntimeOrigin::root(), 5, 10));

        let config = Presence::quorum_config();
        assert_eq!(config.threshold, 5);
        assert_eq!(config.total, 10);
    });
}

#[test]
fn set_quorum_config_invalid() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Presence::set_quorum_config(RuntimeOrigin::root(), 10, 5),
            Error::<Test>::InvalidQuorumConfig
        );
    });
}

#[test]
fn multiple_actors_same_epoch() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(2), epoch));
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(3), epoch));

        assert_eq!(Presence::presence_count(epoch), 3);

        let actor1 = account_to_actor(1);
        let actor2 = account_to_actor(2);
        let actor3 = account_to_actor(3);

        assert!(Presence::presences(epoch, actor1).is_some());
        assert!(Presence::presences(epoch, actor2).is_some());
        assert!(Presence::presences(epoch, actor3).is_some());
    });
}

#[test]
fn same_actor_different_epochs() {
    new_test_ext().execute_with(|| {
        let epoch1 = EpochId::new(1);
        let epoch2 = EpochId::new(2);

        pallet_presence::EpochActive::<Test>::insert(epoch2, true);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch1));
        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch2));

        let actor = account_to_actor(1);

        assert!(Presence::presences(epoch1, actor).is_some());
        assert!(Presence::presences(epoch2, actor).is_some());
    });
}

#[test]
fn negative_vote_does_not_increase_count() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));

        setup_validator(10);
        let actor = account_to_actor(1);

        assert_ok!(Presence::vote_presence(
            RuntimeOrigin::signed(10),
            actor,
            epoch,
            false
        ));

        let record = Presence::presences(epoch, actor).expect("presence should exist");
        assert_eq!(record.vote_count, 0);
    });
}

#[test]
fn events_emitted_correctly() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert_ok!(Presence::declare_presence(RuntimeOrigin::signed(1), epoch));

        let actor = account_to_actor(1);

        System::assert_has_event(RuntimeEvent::Presence(Event::PresenceDeclared {
            actor,
            epoch,
            block_number: 1,
        }));
    });
}

fn run_to_block(n: u64) {
    while System::block_number() < n {
        System::set_block_number(System::block_number() + 1);
    }
}

fn compute_test_commitment(actor: &ActorId, epoch: &EpochId, secret: &[u8; 32], randomness: &[u8; 32]) -> Commitment {
    use seveny_primitives::crypto::DOMAIN_PRESENCE;

    let mut preimage = Vec::with_capacity(DOMAIN_PRESENCE.len() + 32 + 8 + 32 + 32);
    preimage.extend_from_slice(DOMAIN_PRESENCE);
    preimage.extend_from_slice(actor.as_bytes());
    preimage.extend_from_slice(&epoch.inner().to_le_bytes());
    preimage.extend_from_slice(secret);
    preimage.extend_from_slice(randomness);

    let hash = BlakeTwo256::hash(&preimage);
    Commitment(H256(hash.0))
}

#[test]
fn commitment_submitted_event_emitted() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let commitment = Commitment(H256([1u8; 32]));

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        let actor = account_to_actor(1);

        System::assert_has_event(RuntimeEvent::Presence(Event::CommitmentSubmitted {
            actor,
            epoch,
            block_number: 1,
        }));
    });
}

#[test]
fn declaration_phase_commit_initially() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        let phase = Presence::get_declaration_phase(epoch, 1);
        assert_eq!(phase, pallet_presence::DeclarationPhase::Commit);
    });
}

#[test]
fn declaration_phase_transitions_to_reveal() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let commitment = Commitment(H256([1u8; 32]));

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        let phase = Presence::get_declaration_phase(epoch, 1);
        assert_eq!(phase, pallet_presence::DeclarationPhase::Commit);

        let phase = Presence::get_declaration_phase(epoch, 11);
        assert_eq!(phase, pallet_presence::DeclarationPhase::Reveal);
    });
}

#[test]
fn declaration_phase_transitions_to_closed() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let commitment = Commitment(H256([1u8; 32]));

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        let phase = Presence::get_declaration_phase(epoch, 31);
        assert_eq!(phase, pallet_presence::DeclarationPhase::Closed);
    });
}

#[test]
fn reveal_commitment_success() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let secret = [42u8; 32];
        let randomness = [99u8; 32];
        let actor = account_to_actor(1);

        let commitment = compute_test_commitment(&actor, &epoch, &secret, &randomness);

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        run_to_block(12);

        assert_ok!(Presence::reveal_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            secret,
            randomness
        ));

        let declaration = Presence::declarations(epoch, actor).expect("declaration should exist");
        assert!(declaration.revealed);
        assert_eq!(declaration.reveal_block, Some(12));

        System::assert_has_event(RuntimeEvent::Presence(Event::CommitmentRevealed {
            actor,
            epoch,
            block_number: 12,
        }));
    });
}

#[test]
fn reveal_commitment_fails_before_reveal_window() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let secret = [42u8; 32];
        let randomness = [99u8; 32];
        let actor = account_to_actor(1);

        let commitment = compute_test_commitment(&actor, &epoch, &secret, &randomness);

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        assert_noop!(
            Presence::reveal_commitment(RuntimeOrigin::signed(1), epoch, secret, randomness),
            Error::<Test>::NotInRevealPhase
        );
    });
}

#[test]
fn reveal_commitment_fails_after_reveal_window() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let secret = [42u8; 32];
        let randomness = [99u8; 32];
        let actor = account_to_actor(1);

        let commitment = compute_test_commitment(&actor, &epoch, &secret, &randomness);

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        run_to_block(35);

        assert_noop!(
            Presence::reveal_commitment(RuntimeOrigin::signed(1), epoch, secret, randomness),
            Error::<Test>::NotInRevealPhase
        );
    });
}

#[test]
fn reveal_commitment_fails_with_wrong_secret() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let secret = [42u8; 32];
        let randomness = [99u8; 32];
        let wrong_secret = [1u8; 32];
        let actor = account_to_actor(1);

        let commitment = compute_test_commitment(&actor, &epoch, &secret, &randomness);

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        run_to_block(12);

        assert_noop!(
            Presence::reveal_commitment(RuntimeOrigin::signed(1), epoch, wrong_secret, randomness),
            Error::<Test>::CommitmentMismatch
        );
    });
}

#[test]
fn reveal_commitment_fails_with_wrong_randomness() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let secret = [42u8; 32];
        let randomness = [99u8; 32];
        let wrong_randomness = [1u8; 32];
        let actor = account_to_actor(1);

        let commitment = compute_test_commitment(&actor, &epoch, &secret, &randomness);

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        run_to_block(12);

        assert_noop!(
            Presence::reveal_commitment(RuntimeOrigin::signed(1), epoch, secret, wrong_randomness),
            Error::<Test>::CommitmentMismatch
        );
    });
}

#[test]
fn reveal_commitment_fails_if_already_revealed() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let secret = [42u8; 32];
        let randomness = [99u8; 32];
        let actor = account_to_actor(1);

        let commitment = compute_test_commitment(&actor, &epoch, &secret, &randomness);

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        run_to_block(12);

        assert_ok!(Presence::reveal_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            secret,
            randomness
        ));

        assert_noop!(
            Presence::reveal_commitment(RuntimeOrigin::signed(1), epoch, secret, randomness),
            Error::<Test>::AlreadyRevealed
        );
    });
}

#[test]
fn reveal_commitment_fails_without_declaration() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let secret = [42u8; 32];
        let randomness = [99u8; 32];

        pallet_presence::EpochCommitStart::<Test>::insert(epoch, 1u64);

        run_to_block(12);

        assert_noop!(
            Presence::reveal_commitment(RuntimeOrigin::signed(1), epoch, secret, randomness),
            Error::<Test>::DeclarationNotFound
        );
    });
}

#[test]
fn commitment_and_reveal_counts_tracked() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let secret1 = [42u8; 32];
        let randomness1 = [99u8; 32];
        let actor1 = account_to_actor(1);

        let commitment1 = compute_test_commitment(&actor1, &epoch, &secret1, &randomness1);

        let secret2 = [43u8; 32];
        let randomness2 = [100u8; 32];
        let actor2 = account_to_actor(2);

        let commitment2 = compute_test_commitment(&actor2, &epoch, &secret2, &randomness2);

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment1
        ));
        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(2),
            epoch,
            commitment2
        ));

        assert_eq!(Presence::commitment_count(epoch), 2);

        run_to_block(12);

        assert_ok!(Presence::reveal_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            secret1,
            randomness1
        ));

        assert_eq!(Presence::reveal_count(epoch), 1);

        assert_ok!(Presence::reveal_commitment(
            RuntimeOrigin::signed(2),
            epoch,
            secret2,
            randomness2
        ));

        assert_eq!(Presence::reveal_count(epoch), 2);
    });
}

#[test]
fn get_reveal_window_returns_correct_bounds() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);
        let commitment = Commitment(H256([1u8; 32]));

        assert!(Presence::get_reveal_window(epoch).is_none());

        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        let window = Presence::get_reveal_window(epoch).expect("window should exist");
        assert_eq!(window.0, 11);
        assert_eq!(window.1, 31);
    });
}

#[test]
fn is_in_commit_phase_helper() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert!(Presence::is_in_commit_phase(epoch));

        let commitment = Commitment(H256([1u8; 32]));
        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        assert!(Presence::is_in_commit_phase(epoch));

        run_to_block(12);

        assert!(!Presence::is_in_commit_phase(epoch));
    });
}

#[test]
fn is_in_reveal_phase_helper() {
    new_test_ext().execute_with(|| {
        let epoch = EpochId::new(1);

        assert!(!Presence::is_in_reveal_phase(epoch));

        let commitment = Commitment(H256([1u8; 32]));
        assert_ok!(Presence::declare_presence_with_commitment(
            RuntimeOrigin::signed(1),
            epoch,
            commitment
        ));

        run_to_block(12);

        assert!(Presence::is_in_reveal_phase(epoch));

        run_to_block(35);

        assert!(!Presence::is_in_reveal_phase(epoch));
    });
}
