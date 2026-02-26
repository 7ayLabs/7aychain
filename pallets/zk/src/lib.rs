#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::expect_used)]
extern crate alloc;

pub use pallet::*;
pub mod migration;
pub mod verifier;
pub mod weights;

#[cfg(feature = "groth16")]
pub mod groth16;

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "groth16"))]
mod groth16_tests;

use alloc::vec::Vec;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use seveny_primitives::{
    crypto::{hash_with_domain, Nullifier, StateRoot, DOMAIN_NULLIFIER},
    types::ActorId,
};
use sp_core::{blake2_256, H256};

pub use verifier::{
    AcceptAllVerifier, ConfigurableVerifier, NullVerifier, StubVerifier, ZkVerifier,
};

#[cfg(feature = "groth16")]
pub use groth16::Groth16Verifier;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum ProofType {
    #[default]
    Share,
    Presence,
    Access,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum VerificationStatus {
    #[default]
    Pending,
    Verified,
    Rejected,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct ShareStatement {
    pub commitment_hash: H256,
}

#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo,
)]
pub struct ShareWitness {
    pub share_value: [u8; 32],
    pub share_index: u8,
    pub randomness: [u8; 32],
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct PresenceStatement {
    pub epoch_id: u64,
    pub state_root: StateRoot,
    pub nullifier: Nullifier,
}

#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo,
)]
pub struct PresenceWitness {
    pub secret_commitment: H256,
    pub randomness: [u8; 32],
    pub merkle_path: Vec<H256>,
    pub leaf_index: u64,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct AccessStatement {
    pub vault_id: u64,
    pub access_hash: H256,
}

#[derive(
    Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo,
)]
pub struct AccessWitness {
    pub actor_id: ActorId,
    pub ring_position: u32,
    pub membership_commitment: H256,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct ZkProof<T: Config> {
    pub proof_type: ProofType,
    pub proof_data: BoundedVec<u8, T::MaxProofSize>,
    pub created_at: BlockNumberFor<T>,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
#[scale_info(skip_type_params(T))]
pub struct VerificationRecord<T: Config> {
    pub proof_type: ProofType,
    pub statement_hash: H256,
    pub status: VerificationStatus,
    pub verified_at: BlockNumberFor<T>,
    pub verifier: ActorId,
}

pub const DOMAIN_SHARE_PROOF: &[u8] = b"7ay:share:v1";
pub const DOMAIN_ACCESS_PROOF: &[u8] = b"7ay:access:v1";

/// SNARK proof system type for future ZK integration
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum SnarkProofType {
    /// Groth16 - smallest proofs, trusted setup required
    #[default]
    Groth16,
    /// PlonK - universal setup, larger proofs
    PlonK,
    /// Halo2 - no trusted setup, recursive friendly
    Halo2,
}

/// Circuit data stored in the registry
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct CircuitData<BlockNumber> {
    /// The type of SNARK proof system used
    pub proof_type: SnarkProofType,
    /// Hash of the verification key
    pub vk_hash: H256,
    /// When the circuit was registered
    pub registered_at: BlockNumber,
    /// Circuit version (monotonically increasing per circuit_id)
    pub version: u32,
    /// Whether this circuit is active (can be used for verification)
    pub active: bool,
}

/// Maximum size of verification key data
pub type MaxVkSize = ConstU32<4096>;

/// Minimum size of verification key data (prevents empty/trivial VKs)
pub const MIN_VK_SIZE: u32 = 32;

/// Maximum number of public inputs
pub type MaxPublicInputs = ConstU32<16>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    pub use crate::weights::WeightInfo;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
        type WeightInfo: WeightInfo;

        /// The ZK verifier implementation used by this pallet.
        /// Use `StubVerifier` for development, `NullVerifier` to disable,
        /// or a real verifier (e.g. Groth16Verifier) for production.
        type Verifier: ZkVerifier;

        #[pallet::constant]
        type MaxProofSize: Get<u32>;

        #[pallet::constant]
        type MaxVerificationsPerBlock: Get<u32>;

        /// Maximum number of circuits in the registry
        #[pallet::constant]
        type MaxCircuits: Get<u32>;
    }

    #[pallet::storage]
    #[pallet::getter(fn verification_count)]
    pub type VerificationCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn nullifiers)]
    pub type Nullifiers<T: Config> = StorageMap<_, Blake2_128Concat, Nullifier, BlockNumberFor<T>>;

    #[pallet::storage]
    #[pallet::getter(fn verifications)]
    pub type Verifications<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, VerificationRecord<T>>;

    #[pallet::storage]
    #[pallet::getter(fn verified_share_proofs)]
    pub type VerifiedShareProofs<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, BlockNumberFor<T>>;

    #[pallet::storage]
    #[pallet::getter(fn verified_access_proofs)]
    pub type VerifiedAccessProofs<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, u64, Blake2_128Concat, H256, BlockNumberFor<T>>;

    #[pallet::storage]
    #[pallet::getter(fn verifications_this_block)]
    pub type VerificationsThisBlock<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn trusted_verifiers)]
    pub type TrustedVerifiers<T: Config> =
        StorageMap<_, Blake2_128Concat, ActorId, bool, ValueQuery>;

    /// Registry of SNARK circuits and their verification keys
    #[pallet::storage]
    #[pallet::getter(fn circuit_registry)]
    pub type CircuitRegistry<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, CircuitData<BlockNumberFor<T>>>;

    /// Number of registered circuits (active + inactive)
    #[pallet::storage]
    #[pallet::getter(fn circuit_count)]
    pub type CircuitCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// Verification key storage (circuit_id -> vk data)
    #[pallet::storage]
    #[pallet::getter(fn verification_keys)]
    pub type VerificationKeys<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, BoundedVec<u8, MaxVkSize>>;

    /// Hashes of successfully verified SNARK proofs (replay protection)
    #[pallet::storage]
    #[pallet::getter(fn verified_proof_hashes)]
    pub type VerifiedProofHashes<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, BlockNumberFor<T>>;

    /// Current proof system operating mode.
    /// Controls whether stub proofs, SNARK proofs, or both are accepted.
    /// Transitions are monotonic: Legacy -> Transitional -> SnarkOnly.
    #[pallet::storage]
    #[pallet::getter(fn proof_system_mode)]
    pub type CurrentProofSystemMode<T> = StorageValue<_, migration::ProofSystemMode, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            VerificationCount::<T>::put(0u64);
            VerificationsThisBlock::<T>::put(0u32);
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            VerificationsThisBlock::<T>::put(0u32);
            Weight::from_parts(1_000, 0)
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ShareProofVerified {
            statement_hash: H256,
            verifier: ActorId,
        },
        PresenceProofVerified {
            epoch_id: u64,
            nullifier: Nullifier,
            verifier: ActorId,
        },
        AccessProofVerified {
            vault_id: u64,
            access_hash: H256,
            verifier: ActorId,
        },
        ProofRejected {
            proof_type: ProofType,
            statement_hash: H256,
        },
        NullifierConsumed {
            nullifier: Nullifier,
        },
        TrustedVerifierAdded {
            verifier: ActorId,
        },
        TrustedVerifierRemoved {
            verifier: ActorId,
        },
        /// A SNARK circuit was registered
        CircuitRegistered {
            circuit_id: H256,
            proof_type: SnarkProofType,
        },
        /// A SNARK proof was verified
        SnarkVerified {
            circuit_id: H256,
            verifier: ActorId,
        },
        /// Proof system mode was transitioned
        ProofSystemModeChanged {
            from: migration::ProofSystemMode,
            to: migration::ProofSystemMode,
        },
        /// A circuit was deregistered (set to inactive)
        CircuitDeregistered {
            circuit_id: H256,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidProofSize,
        InvalidProofData,
        ProofVerificationFailed,
        NullifierAlreadyUsed,
        TooManyVerifications,
        NotTrustedVerifier,
        StatementAlreadyVerified,
        ProofNotFound,
        /// Circuit not found in registry
        CircuitNotFound,
        /// Circuit already registered
        CircuitAlreadyRegistered,
        /// SNARK verification failed
        SnarkVerificationFailed,
        /// Invalid proof system mode transition (must be forward-only)
        InvalidModeTransition,
        /// Verification key too small or invalid format
        InvalidVerificationKey,
        /// Circuit is not active (deregistered)
        CircuitNotActive,
        /// Current proof system mode does not accept stub/hash-based proofs
        ProofSystemModeRejectsStubProofs,
        /// Current proof system mode does not accept SNARK proofs
        ProofSystemModeRejectsSnarkProofs,
        /// Circuit registry is full (MaxCircuits reached)
        CircuitRegistryFull,
        /// This proof has already been verified (replay protection)
        ProofAlreadyVerified,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::verify_share_proof())]
        pub fn verify_share_proof(
            origin: OriginFor<T>,
            statement: ShareStatement,
            proof: BoundedVec<u8, T::MaxProofSize>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mode = CurrentProofSystemMode::<T>::get();
            ensure!(
                mode.accepts_stub_proofs(),
                Error::<T>::ProofSystemModeRejectsStubProofs
            );

            Self::check_verification_limit()?;

            let statement_hash = Self::hash_statement(&statement.encode());
            ensure!(
                !Verifications::<T>::contains_key(statement_hash),
                Error::<T>::StatementAlreadyVerified
            );

            let verified = T::Verifier::verify_share_proof(&statement, &proof);
            ensure!(verified, Error::<T>::ProofVerificationFailed);

            let block_number = frame_system::Pallet::<T>::block_number();
            let actor = Self::account_to_actor(who);

            let record = VerificationRecord {
                proof_type: ProofType::Share,
                statement_hash,
                status: VerificationStatus::Verified,
                verified_at: block_number,
                verifier: actor,
            };

            Verifications::<T>::insert(statement_hash, record);
            VerifiedShareProofs::<T>::insert(statement.commitment_hash, block_number);
            VerificationCount::<T>::mutate(|c| *c = c.saturating_add(1));
            VerificationsThisBlock::<T>::mutate(|c| *c = c.saturating_add(1));

            Self::deposit_event(Event::ShareProofVerified {
                statement_hash,
                verifier: actor,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::verify_presence_proof())]
        pub fn verify_presence_proof(
            origin: OriginFor<T>,
            statement: PresenceStatement,
            proof: BoundedVec<u8, T::MaxProofSize>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mode = CurrentProofSystemMode::<T>::get();
            ensure!(
                mode.accepts_stub_proofs(),
                Error::<T>::ProofSystemModeRejectsStubProofs
            );

            Self::check_verification_limit()?;

            ensure!(
                !Nullifiers::<T>::contains_key(statement.nullifier),
                Error::<T>::NullifierAlreadyUsed
            );

            let statement_hash = Self::hash_statement(&statement.encode());
            ensure!(
                !Verifications::<T>::contains_key(statement_hash),
                Error::<T>::StatementAlreadyVerified
            );

            let verified = T::Verifier::verify_presence_proof(&statement, &proof);
            ensure!(verified, Error::<T>::ProofVerificationFailed);

            let block_number = frame_system::Pallet::<T>::block_number();
            let actor = Self::account_to_actor(who);

            Nullifiers::<T>::insert(statement.nullifier, block_number);

            let record = VerificationRecord {
                proof_type: ProofType::Presence,
                statement_hash,
                status: VerificationStatus::Verified,
                verified_at: block_number,
                verifier: actor,
            };

            Verifications::<T>::insert(statement_hash, record);
            VerificationCount::<T>::mutate(|c| *c = c.saturating_add(1));
            VerificationsThisBlock::<T>::mutate(|c| *c = c.saturating_add(1));

            Self::deposit_event(Event::NullifierConsumed {
                nullifier: statement.nullifier,
            });

            Self::deposit_event(Event::PresenceProofVerified {
                epoch_id: statement.epoch_id,
                nullifier: statement.nullifier,
                verifier: actor,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::verify_access_proof())]
        pub fn verify_access_proof(
            origin: OriginFor<T>,
            statement: AccessStatement,
            proof: BoundedVec<u8, T::MaxProofSize>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let mode = CurrentProofSystemMode::<T>::get();
            ensure!(
                mode.accepts_stub_proofs(),
                Error::<T>::ProofSystemModeRejectsStubProofs
            );

            Self::check_verification_limit()?;

            let statement_hash = Self::hash_statement(&statement.encode());
            ensure!(
                !Verifications::<T>::contains_key(statement_hash),
                Error::<T>::StatementAlreadyVerified
            );

            let verified = T::Verifier::verify_access_proof(&statement, &proof);
            ensure!(verified, Error::<T>::ProofVerificationFailed);

            let block_number = frame_system::Pallet::<T>::block_number();
            let actor = Self::account_to_actor(who);

            let record = VerificationRecord {
                proof_type: ProofType::Access,
                statement_hash,
                status: VerificationStatus::Verified,
                verified_at: block_number,
                verifier: actor,
            };

            Verifications::<T>::insert(statement_hash, record);
            VerifiedAccessProofs::<T>::insert(
                statement.vault_id,
                statement.access_hash,
                block_number,
            );
            VerificationCount::<T>::mutate(|c| *c = c.saturating_add(1));
            VerificationsThisBlock::<T>::mutate(|c| *c = c.saturating_add(1));

            Self::deposit_event(Event::AccessProofVerified {
                vault_id: statement.vault_id,
                access_hash: statement.access_hash,
                verifier: actor,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::add_trusted_verifier())]
        pub fn add_trusted_verifier(origin: OriginFor<T>, verifier: ActorId) -> DispatchResult {
            ensure_root(origin)?;

            TrustedVerifiers::<T>::insert(verifier, true);

            Self::deposit_event(Event::TrustedVerifierAdded { verifier });

            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::remove_trusted_verifier())]
        pub fn remove_trusted_verifier(origin: OriginFor<T>, verifier: ActorId) -> DispatchResult {
            ensure_root(origin)?;

            TrustedVerifiers::<T>::remove(verifier);

            Self::deposit_event(Event::TrustedVerifierRemoved { verifier });

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::consume_nullifier())]
        pub fn consume_nullifier(origin: OriginFor<T>, nullifier: Nullifier) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(who);

            ensure!(
                TrustedVerifiers::<T>::get(actor),
                Error::<T>::NotTrustedVerifier
            );

            ensure!(
                !Nullifiers::<T>::contains_key(nullifier),
                Error::<T>::NullifierAlreadyUsed
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            Nullifiers::<T>::insert(nullifier, block_number);

            Self::deposit_event(Event::NullifierConsumed { nullifier });

            Ok(())
        }

        /// Register a SNARK circuit with its verification key (root only).
        /// This establishes the upgrade path from hash-based proofs to true ZK.
        /// VK must be at least MIN_VK_SIZE bytes (prevents trivial/empty VKs).
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::register_circuit())]
        pub fn register_circuit(
            origin: OriginFor<T>,
            circuit_id: H256,
            proof_type: SnarkProofType,
            vk: BoundedVec<u8, MaxVkSize>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let count = CircuitCount::<T>::get();
            ensure!(
                count < T::MaxCircuits::get(),
                Error::<T>::CircuitRegistryFull
            );

            ensure!(
                !CircuitRegistry::<T>::contains_key(circuit_id),
                Error::<T>::CircuitAlreadyRegistered
            );

            ensure!(
                vk.len() >= MIN_VK_SIZE as usize,
                Error::<T>::InvalidVerificationKey
            );

            let vk_hash = H256(blake2_256(&vk));
            let block_number = frame_system::Pallet::<T>::block_number();

            let circuit_data = CircuitData {
                proof_type,
                vk_hash,
                registered_at: block_number,
                version: 1,
                active: true,
            };

            CircuitRegistry::<T>::insert(circuit_id, circuit_data);
            VerificationKeys::<T>::insert(circuit_id, vk);
            CircuitCount::<T>::put(count.saturating_add(1));

            Self::deposit_event(Event::CircuitRegistered {
                circuit_id,
                proof_type,
            });

            Ok(())
        }

        /// Verify a SNARK proof against a registered circuit.
        /// Delegates to `T::Verifier::verify_snark` which performs real
        /// cryptographic verification (Groth16 BN254 pairing check in production).
        /// SECURITY: Restricted to trusted verifiers for operational safety.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::verify_snark())]
        pub fn verify_snark(
            origin: OriginFor<T>,
            circuit_id: H256,
            proof: BoundedVec<u8, T::MaxProofSize>,
            inputs: BoundedVec<[u8; 32], MaxPublicInputs>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(who);

            // Restrict to trusted verifiers since stubs are not cryptographic
            ensure!(
                TrustedVerifiers::<T>::get(actor),
                Error::<T>::NotTrustedVerifier
            );

            let mode = CurrentProofSystemMode::<T>::get();
            ensure!(
                mode.accepts_snark_proofs(),
                Error::<T>::ProofSystemModeRejectsSnarkProofs
            );

            Self::check_verification_limit()?;

            // Replay protection: reject already-verified proofs
            let proof_hash = H256(blake2_256(&proof));
            ensure!(
                !VerifiedProofHashes::<T>::contains_key(proof_hash),
                Error::<T>::ProofAlreadyVerified
            );

            let circuit =
                CircuitRegistry::<T>::get(circuit_id).ok_or(Error::<T>::CircuitNotFound)?;
            ensure!(circuit.active, Error::<T>::CircuitNotActive);

            let vk = VerificationKeys::<T>::get(circuit_id).ok_or(Error::<T>::CircuitNotFound)?;

            let verified = T::Verifier::verify_snark(&proof, &inputs, &vk);

            ensure!(verified, Error::<T>::SnarkVerificationFailed);

            let block_number = frame_system::Pallet::<T>::block_number();
            VerifiedProofHashes::<T>::insert(proof_hash, block_number);
            VerificationsThisBlock::<T>::mutate(|c| *c = c.saturating_add(1));
            VerificationCount::<T>::mutate(|c| *c = c.saturating_add(1));

            Self::deposit_event(Event::SnarkVerified {
                circuit_id,
                verifier: actor,
            });

            Ok(())
        }

        /// Deregister a SNARK circuit (root only).
        /// Sets the circuit to inactive. Does not delete storage to preserve
        /// audit trail. Inactive circuits cannot be used for verification.
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::deregister_circuit())]
        pub fn deregister_circuit(origin: OriginFor<T>, circuit_id: H256) -> DispatchResult {
            ensure_root(origin)?;

            CircuitRegistry::<T>::try_mutate(circuit_id, |maybe_circuit| {
                let circuit = maybe_circuit.as_mut().ok_or(Error::<T>::CircuitNotFound)?;
                ensure!(circuit.active, Error::<T>::CircuitNotActive);
                circuit.active = false;
                Ok::<(), DispatchError>(())
            })?;

            CircuitCount::<T>::mutate(|c| *c = c.saturating_sub(1));

            Self::deposit_event(Event::CircuitDeregistered { circuit_id });

            Ok(())
        }

        /// Transition the proof system mode (root only).
        /// Mode transitions are monotonic: Legacy -> Transitional -> SnarkOnly.
        /// Cannot downgrade. This is a governance-controlled operation.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::transition_proof_system_mode())]
        pub fn transition_proof_system_mode(
            origin: OriginFor<T>,
            new_mode: migration::ProofSystemMode,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let current = CurrentProofSystemMode::<T>::get();
            ensure!(
                current.can_transition_to(new_mode),
                Error::<T>::InvalidModeTransition
            );

            CurrentProofSystemMode::<T>::put(new_mode);

            Self::deposit_event(Event::ProofSystemModeChanged {
                from: current,
                to: new_mode,
            });

            Ok(())
        }

        /// M21: Emergency revert from SnarkOnly back to Transitional (root only).
        /// Only allowed when current mode is SnarkOnly. This is a safety valve
        /// for situations where SNARK verification becomes unavailable.
        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::transition_proof_system_mode())]
        pub fn emergency_revert_mode(origin: OriginFor<T>) -> DispatchResult {
            ensure_root(origin)?;

            let current = CurrentProofSystemMode::<T>::get();
            ensure!(
                current == migration::ProofSystemMode::SnarkOnly,
                Error::<T>::InvalidModeTransition
            );

            let target = migration::ProofSystemMode::Transitional;
            CurrentProofSystemMode::<T>::put(target);

            Self::deposit_event(Event::ProofSystemModeChanged {
                from: current,
                to: target,
            });

            Ok(())
        }

        /// M24: Prune old nullifiers and verified proof hashes older than a
        /// given block. Bounded to max_entries per call to prevent unbounded
        /// iteration. Root only.
        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::transition_proof_system_mode())]
        pub fn prune_old_proofs(
            origin: OriginFor<T>,
            older_than: BlockNumberFor<T>,
            max_entries: u32,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let mut pruned = 0u32;

            // Prune nullifiers older than the threshold
            let mut to_remove_nullifiers = Vec::new();
            for (nullifier, block) in Nullifiers::<T>::iter() {
                if pruned >= max_entries {
                    break;
                }
                if block < older_than {
                    to_remove_nullifiers.push(nullifier);
                    pruned = pruned.saturating_add(1);
                }
            }
            for nullifier in to_remove_nullifiers {
                Nullifiers::<T>::remove(nullifier);
            }

            // Prune verified proof hashes older than the threshold
            let mut to_remove_proofs = Vec::new();
            for (hash, block) in VerifiedProofHashes::<T>::iter() {
                if pruned >= max_entries {
                    break;
                }
                if block < older_than {
                    to_remove_proofs.push(hash);
                    pruned = pruned.saturating_add(1);
                }
            }
            for hash in to_remove_proofs {
                VerifiedProofHashes::<T>::remove(hash);
            }

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn check_verification_limit() -> DispatchResult {
            let current = VerificationsThisBlock::<T>::get();
            ensure!(
                current < T::MaxVerificationsPerBlock::get(),
                Error::<T>::TooManyVerifications
            );
            Ok(())
        }

        pub fn hash_statement(data: &[u8]) -> H256 {
            hash_with_domain(b"7ay:zk:statement:v1", data)
        }

        fn account_to_actor(account: T::AccountId) -> ActorId {
            let encoded = account.encode();
            let hash = sp_core::blake2_256(&encoded);
            ActorId::from_raw(hash)
        }

        pub fn is_nullifier_used(nullifier: &Nullifier) -> bool {
            Nullifiers::<T>::contains_key(nullifier)
        }

        pub fn is_share_verified(commitment_hash: &H256) -> bool {
            VerifiedShareProofs::<T>::contains_key(commitment_hash)
        }

        pub fn is_access_verified(vault_id: u64, access_hash: &H256) -> bool {
            VerifiedAccessProofs::<T>::contains_key(vault_id, access_hash)
        }

        pub fn get_verification_record(statement_hash: &H256) -> Option<VerificationRecord<T>> {
            Verifications::<T>::get(statement_hash)
        }

        pub fn total_verifications() -> u64 {
            VerificationCount::<T>::get()
        }

        pub fn generate_share_proof(witness: &ShareWitness) -> (ShareStatement, Vec<u8>) {
            let mut input = Vec::with_capacity(DOMAIN_SHARE_PROOF.len() + 65);
            input.extend_from_slice(DOMAIN_SHARE_PROOF);
            input.extend_from_slice(&witness.share_value);
            input.push(witness.share_index);
            input.extend_from_slice(&witness.randomness);
            let commitment_hash = H256(blake2_256(&input));

            let statement = ShareStatement { commitment_hash };

            let mut proof = Vec::with_capacity(65);
            proof.extend_from_slice(&witness.share_value);
            proof.push(witness.share_index);
            proof.extend_from_slice(&witness.randomness);

            (statement, proof)
        }

        pub fn generate_presence_proof(
            secret: &[u8; 32],
            epoch_id: u64,
            state_root: StateRoot,
        ) -> (PresenceStatement, Vec<u8>) {
            let nullifier = Nullifier::derive(secret, epoch_id);

            let statement = PresenceStatement {
                epoch_id,
                state_root,
                nullifier,
            };

            // Proof layout: secret_commitment[32] || nullifier_binding[32] || reserved[16]
            // INV74: Raw secret NEVER appears in proof data — only a commitment.
            // The secret_commitment = H(secret) proves knowledge without exposure.
            // The nullifier_binding = H(nullifier || epoch) binds to statement.
            let secret_commitment = H256(blake2_256(secret));
            let mut nullifier_input = Vec::with_capacity(40);
            nullifier_input.extend_from_slice(nullifier.0.as_bytes());
            nullifier_input.extend_from_slice(&epoch_id.to_le_bytes());
            let nullifier_binding = hash_with_domain(DOMAIN_NULLIFIER, &nullifier_input);

            let mut proof = Vec::with_capacity(80);
            proof.extend_from_slice(secret_commitment.as_bytes());
            proof.extend_from_slice(nullifier_binding.as_bytes());
            proof.extend_from_slice(&[0u8; 16]);

            (statement, proof)
        }

        pub fn generate_access_proof(
            vault_id: u64,
            actor_id: &ActorId,
            ring_position: u32,
            membership_commitment: &H256,
        ) -> (AccessStatement, Vec<u8>) {
            let mut input = Vec::with_capacity(DOMAIN_ACCESS_PROOF.len() + 76);
            input.extend_from_slice(DOMAIN_ACCESS_PROOF);
            input.extend_from_slice(&vault_id.to_le_bytes());
            input.extend_from_slice(actor_id.0.as_bytes());
            input.extend_from_slice(&ring_position.to_le_bytes());
            input.extend_from_slice(membership_commitment.as_bytes());
            let access_hash = H256(blake2_256(&input));

            let statement = AccessStatement {
                vault_id,
                access_hash,
            };

            let mut proof = Vec::with_capacity(68);
            proof.extend_from_slice(actor_id.0.as_bytes());
            proof.extend_from_slice(&ring_position.to_le_bytes());
            proof.extend_from_slice(membership_commitment.as_bytes());

            (statement, proof)
        }
    }
}
