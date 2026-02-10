#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::expect_used)]
extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use alloc::vec::Vec;
    use frame_support::{
        pallet_prelude::*,
        traits::{Get, StorageVersion},
        BoundedVec,
    };
    use frame_system::pallet_prelude::*;
    use seveny_primitives::types::{ValidatorId, ViolationType};

    use crate::WeightInfo;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[derive(
        Clone,
        Copy,
        PartialEq,
        Eq,
        Encode,
        Decode,
        parity_scale_codec::DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    pub struct DisputeId(pub u64);

    impl DisputeId {
        pub const fn new(id: u64) -> Self {
            Self(id)
        }

        pub const fn inner(self) -> u64 {
            self.0
        }
    }

    #[derive(
        Clone,
        Copy,
        PartialEq,
        Eq,
        Encode,
        Decode,
        parity_scale_codec::DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    pub struct EvidenceId(pub u64);

    #[derive(
        Clone,
        Copy,
        PartialEq,
        Eq,
        Encode,
        Decode,
        parity_scale_codec::DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    pub enum DisputeStatus {
        Open,
        UnderReview,
        Resolved,
        Rejected,
    }

    #[derive(
        Clone,
        Copy,
        PartialEq,
        Eq,
        Encode,
        Decode,
        parity_scale_codec::DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    pub enum DisputeOutcome {
        ValidatorSlashed,
        DisputeRejected,
        InsufficientEvidence,
    }

    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        parity_scale_codec::DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct Dispute<T: Config> {
        pub id: DisputeId,
        pub reporter: T::AccountId,
        pub target: ValidatorId,
        pub violation: ViolationType,
        pub status: DisputeStatus,
        pub created_at: BlockNumberFor<T>,
        pub resolved_at: Option<BlockNumberFor<T>>,
        pub outcome: Option<DisputeOutcome>,
        pub evidence_count: u32,
    }

    #[derive(
        Clone,
        PartialEq,
        Eq,
        Encode,
        Decode,
        parity_scale_codec::DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct Evidence<T: Config> {
        pub id: EvidenceId,
        pub dispute_id: DisputeId,
        pub submitter: T::AccountId,
        pub data_hash: sp_core::H256,
        pub submitted_at: BlockNumberFor<T>,
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
        type WeightInfo: WeightInfo;

        #[pallet::constant]
        type MaxEvidencePerDispute: Get<u32>;

        #[pallet::constant]
        type DisputeResolutionPeriod: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MinEvidenceRequired: Get<u32>;

        #[pallet::constant]
        type MaxDisputesPerValidator: Get<u32>;

        #[pallet::constant]
        type MaxOpenDisputes: Get<u32>;
    }

    #[pallet::storage]
    #[pallet::getter(fn disputes)]
    pub type Disputes<T: Config> =
        StorageMap<_, Blake2_128Concat, DisputeId, Dispute<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn evidence)]
    pub type EvidenceStore<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        DisputeId,
        Blake2_128Concat,
        EvidenceId,
        Evidence<T>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn dispute_count)]
    pub type DisputeCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn evidence_count)]
    pub type EvidenceCount<T: Config> = StorageMap<_, Blake2_128Concat, DisputeId, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn disputes_by_validator)]
    pub type DisputesByValidator<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        ValidatorId,
        BoundedVec<DisputeId, T::MaxDisputesPerValidator>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn open_disputes)]
    pub type OpenDisputes<T: Config> =
        StorageValue<_, BoundedVec<DisputeId, T::MaxOpenDisputes>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        DisputeOpened {
            dispute_id: DisputeId,
            reporter: T::AccountId,
            target: ValidatorId,
            violation: ViolationType,
        },
        EvidenceSubmitted {
            dispute_id: DisputeId,
            evidence_id: EvidenceId,
            submitter: T::AccountId,
        },
        DisputeUnderReview {
            dispute_id: DisputeId,
        },
        DisputeResolved {
            dispute_id: DisputeId,
            outcome: DisputeOutcome,
        },
        DisputeRejected {
            dispute_id: DisputeId,
            reason: DisputeRejectionReason,
        },
    }

    #[derive(
        Clone,
        Copy,
        PartialEq,
        Eq,
        Encode,
        Decode,
        parity_scale_codec::DecodeWithMemTracking,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebug,
    )]
    pub enum DisputeRejectionReason {
        InsufficientEvidence,
        ResolutionPeriodExpired,
        InvalidTarget,
    }

    #[pallet::error]
    pub enum Error<T> {
        DisputeNotFound,
        DisputeAlreadyResolved,
        DisputeNotOpen,
        MaxEvidenceReached,
        EvidenceNotFound,
        NotAuthorized,
        InvalidViolationType,
        ResolutionPeriodNotElapsed,
        ResolutionPeriodExpired,
        InsufficientEvidence,
        TargetNotValidator,
        MaxDisputesForValidatorReached,
        MaxOpenDisputesReached,
    }

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            DisputeCount::<T>::put(0u64);
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::open_dispute())]
        pub fn open_dispute(
            origin: OriginFor<T>,
            target: ValidatorId,
            violation: ViolationType,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let dispute_id = DisputeId::new(DisputeCount::<T>::get());
            DisputeCount::<T>::put(dispute_id.inner().saturating_add(1));

            let dispute = Dispute {
                id: dispute_id,
                reporter: who.clone(),
                target,
                violation,
                status: DisputeStatus::Open,
                created_at: block_number,
                resolved_at: None,
                outcome: None,
                evidence_count: 0,
            };

            Disputes::<T>::insert(dispute_id, dispute);

            DisputesByValidator::<T>::try_mutate(target, |disputes| {
                disputes
                    .try_push(dispute_id)
                    .map_err(|_| Error::<T>::MaxDisputesForValidatorReached)
            })?;

            OpenDisputes::<T>::try_mutate(|disputes| {
                disputes
                    .try_push(dispute_id)
                    .map_err(|_| Error::<T>::MaxOpenDisputesReached)
            })?;

            Self::deposit_event(Event::DisputeOpened {
                dispute_id,
                reporter: who,
                target,
                violation,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::submit_evidence())]
        pub fn submit_evidence(
            origin: OriginFor<T>,
            dispute_id: DisputeId,
            data_hash: sp_core::H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut dispute = Disputes::<T>::get(dispute_id).ok_or(Error::<T>::DisputeNotFound)?;

            ensure!(
                dispute.status == DisputeStatus::Open
                    || dispute.status == DisputeStatus::UnderReview,
                Error::<T>::DisputeAlreadyResolved
            );
            ensure!(
                dispute.evidence_count < T::MaxEvidencePerDispute::get(),
                Error::<T>::MaxEvidenceReached
            );

            let evidence_id = EvidenceId(EvidenceCount::<T>::get(dispute_id));
            EvidenceCount::<T>::insert(dispute_id, evidence_id.0.saturating_add(1));

            let evidence = Evidence {
                id: evidence_id,
                dispute_id,
                submitter: who.clone(),
                data_hash,
                submitted_at: block_number,
            };

            EvidenceStore::<T>::insert(dispute_id, evidence_id, evidence);

            dispute.evidence_count = dispute.evidence_count.saturating_add(1);

            if dispute.evidence_count >= T::MinEvidenceRequired::get()
                && dispute.status == DisputeStatus::Open
            {
                dispute.status = DisputeStatus::UnderReview;
                Self::deposit_event(Event::DisputeUnderReview { dispute_id });
            }

            Disputes::<T>::insert(dispute_id, dispute);

            Self::deposit_event(Event::EvidenceSubmitted {
                dispute_id,
                evidence_id,
                submitter: who,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::resolve_dispute())]
        pub fn resolve_dispute(
            origin: OriginFor<T>,
            dispute_id: DisputeId,
            outcome: DisputeOutcome,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut dispute = Disputes::<T>::get(dispute_id).ok_or(Error::<T>::DisputeNotFound)?;

            ensure!(
                dispute.status == DisputeStatus::Open
                    || dispute.status == DisputeStatus::UnderReview,
                Error::<T>::DisputeAlreadyResolved
            );

            dispute.status = DisputeStatus::Resolved;
            dispute.resolved_at = Some(block_number);
            dispute.outcome = Some(outcome);

            Disputes::<T>::insert(dispute_id, dispute);

            OpenDisputes::<T>::mutate(|disputes| {
                disputes.retain(|id| *id != dispute_id);
            });

            Self::deposit_event(Event::DisputeResolved {
                dispute_id,
                outcome,
            });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::reject_dispute())]
        pub fn reject_dispute(
            origin: OriginFor<T>,
            dispute_id: DisputeId,
            reason: DisputeRejectionReason,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let mut dispute = Disputes::<T>::get(dispute_id).ok_or(Error::<T>::DisputeNotFound)?;

            ensure!(
                dispute.status == DisputeStatus::Open
                    || dispute.status == DisputeStatus::UnderReview,
                Error::<T>::DisputeAlreadyResolved
            );

            dispute.status = DisputeStatus::Rejected;
            dispute.resolved_at = Some(block_number);
            dispute.outcome = Some(DisputeOutcome::DisputeRejected);

            Disputes::<T>::insert(dispute_id, dispute);

            OpenDisputes::<T>::mutate(|disputes| {
                disputes.retain(|id| *id != dispute_id);
            });

            Self::deposit_event(Event::DisputeRejected { dispute_id, reason });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn get_dispute(dispute_id: DisputeId) -> Option<Dispute<T>> {
            Disputes::<T>::get(dispute_id)
        }

        pub fn get_evidence(dispute_id: DisputeId, evidence_id: EvidenceId) -> Option<Evidence<T>> {
            EvidenceStore::<T>::get(dispute_id, evidence_id)
        }

        pub fn is_dispute_open(dispute_id: DisputeId) -> bool {
            Disputes::<T>::get(dispute_id)
                .map(|d| d.status == DisputeStatus::Open || d.status == DisputeStatus::UnderReview)
                .unwrap_or(false)
        }

        pub fn get_disputes_for_validator(target: ValidatorId) -> Vec<DisputeId> {
            DisputesByValidator::<T>::get(target).into_inner()
        }

        pub fn get_open_dispute_count() -> u32 {
            OpenDisputes::<T>::get().len() as u32
        }
    }
}
