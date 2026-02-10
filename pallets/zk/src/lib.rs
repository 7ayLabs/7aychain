#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;
pub mod weights;

#[cfg(test)]
mod tests;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use seveny_primitives::{
    crypto::{Nullifier, StateRoot},
    types::ActorId,
};
use sp_core::{blake2_256, H256};
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub enum ProofType {
    Share,
    Presence,
    Access,
}

impl Default for ProofType {
    fn default() -> Self {
        Self::Share
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Rejected,
}

impl Default for VerificationStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub struct ShareStatement {
    pub commitment_hash: H256,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo)]
pub struct ShareWitness {
    pub share_value: [u8; 32],
    pub share_index: u8,
    pub randomness: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub struct PresenceStatement {
    pub epoch_id: u64,
    pub state_root: StateRoot,
    pub nullifier: Nullifier,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo)]
pub struct PresenceWitness {
    pub secret: [u8; 32],
    pub randomness: [u8; 32],
    pub merkle_path: Vec<H256>,
    pub leaf_index: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
pub struct AccessStatement {
    pub vault_id: u64,
    pub access_hash: H256,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo)]
pub struct AccessWitness {
    pub actor_id: ActorId,
    pub ring_position: u32,
    pub membership_commitment: H256,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct ZkProof<T: Config> {
    pub proof_type: ProofType,
    pub proof_data: BoundedVec<u8, T::MaxProofSize>,
    pub created_at: BlockNumberFor<T>,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, parity_scale_codec::DecodeWithMemTracking, TypeInfo, MaxEncodedLen)]
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

pub trait ZkVerifier {
    fn verify_share_proof(statement: &ShareStatement, proof: &[u8]) -> bool;
    fn verify_presence_proof(statement: &PresenceStatement, proof: &[u8]) -> bool;
    fn verify_access_proof(statement: &AccessStatement, proof: &[u8]) -> bool;
}

pub struct SimpleHashVerifier;

impl ZkVerifier for SimpleHashVerifier {
    fn verify_share_proof(statement: &ShareStatement, proof: &[u8]) -> bool {
        if proof.len() < 65 {
            return false;
        }
        let share_value: [u8; 32] = proof[0..32].try_into().unwrap_or([0u8; 32]);
        let share_index = proof[32];
        let randomness: [u8; 32] = proof[33..65].try_into().unwrap_or([0u8; 32]);

        let mut input = Vec::with_capacity(DOMAIN_SHARE_PROOF.len() + 65);
        input.extend_from_slice(DOMAIN_SHARE_PROOF);
        input.extend_from_slice(&share_value);
        input.push(share_index);
        input.extend_from_slice(&randomness);
        let computed = H256(blake2_256(&input));

        computed == statement.commitment_hash
    }

    fn verify_presence_proof(statement: &PresenceStatement, proof: &[u8]) -> bool {
        if proof.len() < 80 {
            return false;
        }
        let secret: [u8; 32] = proof[0..32].try_into().unwrap_or([0u8; 32]);
        let nonce_bytes: [u8; 8] = proof[64..72].try_into().unwrap_or([0u8; 8]);
        let nonce = u64::from_le_bytes(nonce_bytes);

        let derived_nullifier = Nullifier::derive(&secret, statement.epoch_id, nonce);

        derived_nullifier == statement.nullifier
    }

    fn verify_access_proof(statement: &AccessStatement, proof: &[u8]) -> bool {
        if proof.len() < 68 {
            return false;
        }
        let actor_bytes: [u8; 32] = proof[0..32].try_into().unwrap_or([0u8; 32]);
        let ring_position_bytes: [u8; 4] = proof[32..36].try_into().unwrap_or([0u8; 4]);
        let membership: [u8; 32] = proof[36..68].try_into().unwrap_or([0u8; 32]);

        let mut input = Vec::with_capacity(DOMAIN_ACCESS_PROOF.len() + 76);
        input.extend_from_slice(DOMAIN_ACCESS_PROOF);
        input.extend_from_slice(&statement.vault_id.to_le_bytes());
        input.extend_from_slice(&actor_bytes);
        input.extend_from_slice(&ring_position_bytes);
        input.extend_from_slice(&membership);
        let computed = H256(blake2_256(&input));

        computed == statement.access_hash
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    pub use crate::weights::WeightInfo;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
        type WeightInfo: WeightInfo;

        #[pallet::constant]
        type MaxProofSize: Get<u32>;

        #[pallet::constant]
        type MaxVerificationsPerBlock: Get<u32>;
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
    pub type TrustedVerifiers<T: Config> = StorageMap<_, Blake2_128Concat, ActorId, bool>;

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

            Self::check_verification_limit()?;

            let statement_hash = Self::hash_statement(&statement.encode());
            ensure!(
                !Verifications::<T>::contains_key(statement_hash),
                Error::<T>::StatementAlreadyVerified
            );

            let verified = SimpleHashVerifier::verify_share_proof(&statement, &proof);
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

            let verified = SimpleHashVerifier::verify_presence_proof(&statement, &proof);
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

            Self::check_verification_limit()?;

            let statement_hash = Self::hash_statement(&statement.encode());
            ensure!(
                !Verifications::<T>::contains_key(statement_hash),
                Error::<T>::StatementAlreadyVerified
            );

            let verified = SimpleHashVerifier::verify_access_proof(&statement, &proof);
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
            VerifiedAccessProofs::<T>::insert(statement.vault_id, statement.access_hash, block_number);
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
                TrustedVerifiers::<T>::get(actor).unwrap_or(false),
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
            H256(blake2_256(data))
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
            nonce: u64,
            state_root: StateRoot,
        ) -> (PresenceStatement, Vec<u8>) {
            let nullifier = Nullifier::derive(secret, epoch_id, nonce);

            let statement = PresenceStatement {
                epoch_id,
                state_root,
                nullifier,
            };

            let mut proof = Vec::with_capacity(80);
            proof.extend_from_slice(secret);
            proof.extend_from_slice(&[0u8; 32]);
            proof.extend_from_slice(&nonce.to_le_bytes());
            proof.extend_from_slice(&[0u8; 8]);

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
