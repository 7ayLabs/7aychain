#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::expect_used)]

extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Get, StorageVersion},
    };
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
    use scale_info::TypeInfo;
    use seveny_primitives::{
        traits::{
            ActorActivityChecker, CarrierRewardHandler, DeviceEligibilityChecker, EpochProvider,
            PresenceVerifier, ServiceNodePresenceVerifier, ValidatorProvider,
            ValidatorStakeProvider,
        },
        types::{ActorId, EpochId, ValidatorId},
    };
    use sp_core::H256;
    use sp_runtime::traits::Saturating;

    use crate::WeightInfo;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(4);

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
        Default,
    )]
    pub enum NumberStatus {
        #[default]
        Reserved,
        ActivationPending,
        Activated,
        Suspended,
        RecoveryPending,
        Recovered,
        Revoked,
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
        Default,
    )]
    pub enum ServiceRequestKind {
        #[default]
        Activation,
        Recovery,
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
        Default,
    )]
    pub enum ServiceRequestStatus {
        #[default]
        Requested,
        Witnessed,
        Finalized,
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
        Default,
    )]
    pub enum ProvisioningState {
        #[default]
        None,
        Pending,
        Provisioned,
        Failed,
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
        Default,
    )]
    pub enum ServiceNodeStatus {
        #[default]
        Active,
        Suspended,
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
        Default,
    )]
    pub enum CoverageStatus {
        #[default]
        Unavailable,
        Weak,
        Available,
        Strong,
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
    pub struct SubscriberBinding<T: Config> {
        pub owner: ActorId,
        pub current_device_id: Option<u64>,
        pub sim_profile_commitment: Option<H256>,
        pub region_id: H256,
        pub status: NumberStatus,
        pub activation_epoch: Option<EpochId>,
        pub provisioning_state: ProvisioningState,
        pub provisioning_receipt: Option<H256>,
        pub reserved_at: BlockNumberFor<T>,
        pub activated_at: Option<BlockNumberFor<T>>,
        pub service_lease_until: Option<BlockNumberFor<T>>,
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
    pub struct ServiceRequest<T: Config> {
        pub kind: ServiceRequestKind,
        pub actor: ActorId,
        pub number_id: H256,
        pub epoch: EpochId,
        pub region_id: H256,
        pub pending_device_id: u64,
        pub pending_sim_profile_commitment: H256,
        pub witness_count: u32,
        pub status: ServiceRequestStatus,
        pub created_at: BlockNumberFor<T>,
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
    pub struct ServiceWitness<T: Config> {
        pub validator: ValidatorId,
        pub region_id: H256,
        pub signal_score: u8,
        pub attested_at: BlockNumberFor<T>,
    }

    /// Aggregated signal quality metrics for a carrier number.
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
        Default,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct SignalAggregation<T: Config> {
        pub avg_score: u8,
        pub min_score: u8,
        pub max_score: u8,
        pub sample_count: u32,
        pub last_updated: BlockNumberFor<T>,
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
    pub struct ServiceNodeProfile<T: Config> {
        pub controller: T::AccountId,
        pub validator: ValidatorId,
        pub regions: BoundedVec<H256, T::MaxRegionsPerServiceNode>,
        pub status: ServiceNodeStatus,
        pub stake_weight: u128,
        pub last_presence_epoch: Option<EpochId>,
        pub last_signal_score: Option<u8>,
        pub delivery_success_count: u32,
        pub delivery_failure_count: u32,
        pub slash_points: u32,
        pub updated_at: BlockNumberFor<T>,
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
    pub struct RegionCoverageSummary<T: Config> {
        pub epoch: EpochId,
        pub region_id: H256,
        pub registered_node_count: u32,
        pub eligible_node_count: u32,
        pub avg_signal_score: Option<u8>,
        pub status: CoverageStatus,
        pub updated_at: BlockNumberFor<T>,
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
        type WeightInfo: WeightInfo;
        type EpochProvider: EpochProvider;
        type ValidatorProvider: ValidatorProvider;
        type ValidatorStakeProvider: ValidatorStakeProvider;
        type ActorChecker: ActorActivityChecker;
        type DeviceChecker: DeviceEligibilityChecker;
        type PresenceVerifier: PresenceVerifier;
        type ServiceNodePresenceVerifier: ServiceNodePresenceVerifier<Self::AccountId>;
        type RewardHandler: CarrierRewardHandler<Self::AccountId>;

        #[pallet::constant]
        type MaxNumbersPerActor: Get<u32>;

        #[pallet::constant]
        type MaxWitnessesPerRequest: Get<u32>;

        #[pallet::constant]
        type WitnessThreshold: Get<u32>;

        #[pallet::constant]
        type ServiceLeaseBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type WitnessRewardAmount: Get<u128>;

        #[pallet::constant]
        type CarrierBridgeAccount: Get<Self::AccountId>;

        #[pallet::constant]
        type MaxRegionsPerServiceNode: Get<u32>;

        #[pallet::constant]
        type MinServiceStake: Get<u128>;

        #[pallet::constant]
        type MinRegionalServiceNodes: Get<u32>;

        #[pallet::constant]
        type StrongRegionalServiceNodes: Get<u32>;

        #[pallet::constant]
        type ServiceNodeSlashThreshold: Get<u32>;
    }

    #[pallet::storage]
    #[pallet::getter(fn numbers)]
    pub type Numbers<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, SubscriberBinding<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn actor_number_count)]
    pub type ActorNumberCount<T: Config> =
        StorageMap<_, Blake2_128Concat, ActorId, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn actor_numbers)]
    pub type ActorNumbers<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, ActorId, Blake2_128Concat, H256, (), OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn device_numbers)]
    pub type DeviceNumbers<T: Config> = StorageMap<_, Blake2_128Concat, u64, H256, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn service_requests)]
    pub type ServiceRequests<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        EpochId,
        Blake2_128Concat,
        H256,
        ServiceRequest<T>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn current_request_epoch)]
    pub type CurrentRequestEpoch<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, EpochId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn service_witnesses)]
    pub type ServiceWitnesses<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Blake2_128Concat, EpochId>,
            NMapKey<Blake2_128Concat, H256>,
            NMapKey<Blake2_128Concat, ValidatorId>,
        ),
        ServiceWitness<T>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn service_witness_count)]
    pub type ServiceWitnessCount<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, EpochId, Blake2_128Concat, H256, u32, ValueQuery>;

    /// Aggregated signal quality per carrier number.
    #[pallet::storage]
    #[pallet::getter(fn signal_quality)]
    pub type SignalQuality<T: Config> =
        StorageMap<_, Blake2_128Concat, H256, SignalAggregation<T>, OptionQuery>;

    /// Lifetime carrier witness count per validator (leaderboard).
    #[pallet::storage]
    #[pallet::getter(fn validator_witness_count)]
    pub type ValidatorWitnessCount<T: Config> =
        StorageMap<_, Blake2_128Concat, ValidatorId, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn service_nodes)]
    pub type ServiceNodes<T: Config> =
        StorageMap<_, Blake2_128Concat, ValidatorId, ServiceNodeProfile<T>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn region_service_nodes)]
    pub type RegionServiceNodes<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, H256, Blake2_128Concat, ValidatorId, (), OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn region_coverage)]
    pub type RegionCoverage<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        EpochId,
        Blake2_128Concat,
        H256,
        RegionCoverageSummary<T>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn failure_reports)]
    pub type FailureReports<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Blake2_128Concat, EpochId>,
            NMapKey<Blake2_128Concat, H256>,
            NMapKey<Blake2_128Concat, ValidatorId>,
        ),
        (),
        OptionQuery,
    >;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: core::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {}
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        NumberReserved {
            actor: ActorId,
            number_id: H256,
            region_id: H256,
        },
        ServiceActivationRequested {
            actor: ActorId,
            number_id: H256,
            device_id: u64,
            epoch: EpochId,
        },
        RecoveryRequested {
            actor: ActorId,
            number_id: H256,
            replacement_device_id: u64,
            epoch: EpochId,
        },
        ServiceWitnessSubmitted {
            validator: ValidatorId,
            number_id: H256,
            epoch: EpochId,
            signal_score: u8,
            witness_count: u32,
        },
        ServiceActivated {
            actor: ActorId,
            number_id: H256,
            device_id: u64,
            epoch: EpochId,
            lease_until: BlockNumberFor<T>,
        },
        ServiceRecovered {
            actor: ActorId,
            number_id: H256,
            device_id: u64,
            epoch: EpochId,
            lease_until: BlockNumberFor<T>,
        },
        NumberSuspended {
            actor: ActorId,
            number_id: H256,
            reason: H256,
        },
        NumberRevoked {
            actor: ActorId,
            number_id: H256,
        },
        SignalQualityUpdated {
            number_id: H256,
            avg_score: u8,
            sample_count: u32,
        },
        ProvisioningAuthorized {
            actor: ActorId,
            number_id: H256,
            device_id: u64,
            epoch: EpochId,
        },
        ProvisioningRecorded {
            number_id: H256,
            receipt: H256,
        },
        ProvisioningFailed {
            number_id: H256,
            receipt: Option<H256>,
        },
        ServiceNodeRegistered {
            validator: ValidatorId,
            region_count: u32,
            stake_weight: u128,
        },
        ServiceNodeRegionsUpdated {
            validator: ValidatorId,
            region_count: u32,
        },
        ServiceNodeSuspended {
            validator: ValidatorId,
            slash_points: u32,
        },
        ServiceNodeReactivated {
            validator: ValidatorId,
        },
        RegionCoverageUpdated {
            epoch: EpochId,
            region_id: H256,
            eligible_node_count: u32,
            status: CoverageStatus,
            avg_signal_score: Option<u8>,
        },
        ServiceNodeDeliveryRecorded {
            validator: ValidatorId,
            number_id: H256,
            signal_score: u8,
            success_count: u32,
        },
        ServiceNodeFailureReported {
            validator: ValidatorId,
            number_id: H256,
            slash_points: u32,
            suspended: bool,
        },
        WitnessRewardPaid {
            validator: ValidatorId,
            number_id: H256,
            amount: u128,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        NumberAlreadyReserved,
        NumberNotFound,
        MaxNumbersReached,
        NotNumberOwner,
        ActorNotActive,
        DeviceNotEligible,
        PresenceNotVerified,
        EpochNotActive,
        ServiceRequestAlreadyExists,
        ServiceRequestNotFound,
        DuplicateWitness,
        MaxWitnessesReached,
        WitnessThresholdNotMet,
        ValidatorNotActive,
        InvalidSignalScore,
        RegionMismatch,
        InvalidNumberStatus,
        DeviceAlreadyBound,
        UnauthorizedFinalize,
        NumberRevoked,
        NumberSuspended,
        NumberNotActive,
        ProvisioningUpdateNotAllowed,
        ServiceNodeAlreadyRegistered,
        ServiceNodeNotFound,
        NoServiceRegions,
        ServiceNodeNotEligible,
        InsufficientServiceStake,
        InsufficientRegionalCoverage,
        ServiceNodeRegionNotServed,
        DuplicateFailureReport,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::reserve_number())]
        pub fn reserve_number(
            origin: OriginFor<T>,
            number_id: H256,
            region_id: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);

            ensure!(
                T::ActorChecker::is_actor_active(actor),
                Error::<T>::ActorNotActive
            );
            ensure!(
                !Numbers::<T>::contains_key(number_id),
                Error::<T>::NumberAlreadyReserved
            );

            let count = ActorNumberCount::<T>::get(actor);
            ensure!(
                count < T::MaxNumbersPerActor::get(),
                Error::<T>::MaxNumbersReached
            );

            let now = frame_system::Pallet::<T>::block_number();
            let binding = SubscriberBinding::<T> {
                owner: actor,
                current_device_id: None,
                sim_profile_commitment: None,
                region_id,
                status: NumberStatus::Reserved,
                activation_epoch: None,
                provisioning_state: ProvisioningState::None,
                provisioning_receipt: None,
                reserved_at: now,
                activated_at: None,
                service_lease_until: None,
            };

            Numbers::<T>::insert(number_id, binding);
            ActorNumbers::<T>::insert(actor, number_id, ());
            ActorNumberCount::<T>::insert(actor, count.saturating_add(1));

            Self::deposit_event(Event::NumberReserved {
                actor,
                number_id,
                region_id,
            });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::request_activation())]
        pub fn request_activation(
            origin: OriginFor<T>,
            number_id: H256,
            device_id: u64,
            epoch: EpochId,
            sim_profile_commitment: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);

            Self::ensure_actor_service_ready(actor, device_id, epoch)?;

            let mut binding = Numbers::<T>::get(number_id).ok_or(Error::<T>::NumberNotFound)?;
            ensure!(binding.owner == actor, Error::<T>::NotNumberOwner);
            ensure!(
                binding.status != NumberStatus::Revoked,
                Error::<T>::NumberRevoked
            );
            ensure!(
                binding.status == NumberStatus::Reserved
                    || binding.status == NumberStatus::Suspended,
                Error::<T>::InvalidNumberStatus
            );
            ensure!(
                !ServiceRequests::<T>::contains_key(epoch, number_id),
                Error::<T>::ServiceRequestAlreadyExists
            );
            if let Some(bound_number) = DeviceNumbers::<T>::get(device_id) {
                ensure!(bound_number == number_id, Error::<T>::DeviceAlreadyBound);
            }

            let now = frame_system::Pallet::<T>::block_number();
            let coverage = Self::refresh_region_coverage_for(binding.region_id, epoch, now);
            ensure!(
                coverage.eligible_node_count >= T::MinRegionalServiceNodes::get(),
                Error::<T>::InsufficientRegionalCoverage
            );

            let request = ServiceRequest::<T> {
                kind: ServiceRequestKind::Activation,
                actor,
                number_id,
                epoch,
                region_id: binding.region_id,
                pending_device_id: device_id,
                pending_sim_profile_commitment: sim_profile_commitment,
                witness_count: 0,
                status: ServiceRequestStatus::Requested,
                created_at: now,
            };
            binding.status = NumberStatus::ActivationPending;
            binding.provisioning_state = ProvisioningState::None;
            binding.provisioning_receipt = None;
            Numbers::<T>::insert(number_id, binding);
            ServiceRequests::<T>::insert(epoch, number_id, request);
            CurrentRequestEpoch::<T>::insert(number_id, epoch);

            Self::deposit_event(Event::ServiceActivationRequested {
                actor,
                number_id,
                device_id,
                epoch,
            });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::request_recovery())]
        pub fn request_recovery(
            origin: OriginFor<T>,
            number_id: H256,
            replacement_device_id: u64,
            epoch: EpochId,
            replacement_sim_profile_commitment: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);

            Self::ensure_actor_service_ready(actor, replacement_device_id, epoch)?;

            let mut binding = Numbers::<T>::get(number_id).ok_or(Error::<T>::NumberNotFound)?;
            ensure!(binding.owner == actor, Error::<T>::NotNumberOwner);
            ensure!(
                matches!(
                    binding.status,
                    NumberStatus::Activated | NumberStatus::Suspended | NumberStatus::Recovered
                ),
                Error::<T>::InvalidNumberStatus
            );
            ensure!(
                !ServiceRequests::<T>::contains_key(epoch, number_id),
                Error::<T>::ServiceRequestAlreadyExists
            );
            if let Some(bound_number) = DeviceNumbers::<T>::get(replacement_device_id) {
                ensure!(bound_number == number_id, Error::<T>::DeviceAlreadyBound);
            }

            let now = frame_system::Pallet::<T>::block_number();
            let coverage = Self::refresh_region_coverage_for(binding.region_id, epoch, now);
            ensure!(
                coverage.eligible_node_count >= T::MinRegionalServiceNodes::get(),
                Error::<T>::InsufficientRegionalCoverage
            );

            binding.status = NumberStatus::RecoveryPending;
            binding.provisioning_state = ProvisioningState::None;
            binding.provisioning_receipt = None;
            Numbers::<T>::insert(number_id, binding.clone());

            let request = ServiceRequest::<T> {
                kind: ServiceRequestKind::Recovery,
                actor,
                number_id,
                epoch,
                region_id: binding.region_id,
                pending_device_id: replacement_device_id,
                pending_sim_profile_commitment: replacement_sim_profile_commitment,
                witness_count: 0,
                status: ServiceRequestStatus::Requested,
                created_at: now,
            };

            ServiceRequests::<T>::insert(epoch, number_id, request);
            CurrentRequestEpoch::<T>::insert(number_id, epoch);

            Self::deposit_event(Event::RecoveryRequested {
                actor,
                number_id,
                replacement_device_id,
                epoch,
            });
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::submit_service_witness())]
        pub fn submit_service_witness(
            origin: OriginFor<T>,
            number_id: H256,
            epoch: EpochId,
            region_id: H256,
            signal_score: u8,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let validator = Self::account_to_validator(&who);

            ensure!(signal_score > 0, Error::<T>::InvalidSignalScore);
            ensure!(
                T::ValidatorProvider::is_validator_active(validator),
                Error::<T>::ValidatorNotActive
            );

            let mut request = ServiceRequests::<T>::get(epoch, number_id)
                .ok_or(Error::<T>::ServiceRequestNotFound)?;
            ensure!(request.region_id == region_id, Error::<T>::RegionMismatch);
            ensure!(
                !ServiceWitnesses::<T>::contains_key((epoch, number_id, validator)),
                Error::<T>::DuplicateWitness
            );
            ensure!(
                request.witness_count < T::MaxWitnessesPerRequest::get(),
                Error::<T>::MaxWitnessesReached
            );
            Self::ensure_service_node_eligible(validator, &who, region_id, epoch)?;

            let now = frame_system::Pallet::<T>::block_number();
            let witness = ServiceWitness::<T> {
                validator,
                region_id,
                signal_score,
                attested_at: now,
            };
            ServiceWitnesses::<T>::insert((epoch, number_id, validator), witness);

            request.witness_count = request.witness_count.saturating_add(1);
            request.status = ServiceRequestStatus::Witnessed;
            ServiceWitnessCount::<T>::insert(epoch, number_id, request.witness_count);
            ServiceRequests::<T>::insert(epoch, number_id, request.clone());
            ServiceNodes::<T>::mutate(validator, |profile| {
                if let Some(profile) = profile {
                    profile.stake_weight = T::ValidatorStakeProvider::validator_stake(validator);
                    profile.last_presence_epoch = Some(epoch);
                    profile.updated_at = now;
                }
            });

            Self::deposit_event(Event::ServiceWitnessSubmitted {
                validator,
                number_id,
                epoch,
                signal_score,
                witness_count: request.witness_count,
            });
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::finalize_service_request())]
        pub fn finalize_service_request(
            origin: OriginFor<T>,
            number_id: H256,
            epoch: EpochId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let caller_actor = Self::account_to_actor(&who);
            let caller_validator = Self::account_to_validator(&who);

            let request = ServiceRequests::<T>::get(epoch, number_id)
                .ok_or(Error::<T>::ServiceRequestNotFound)?;
            ensure!(
                caller_actor == request.actor
                    || T::ValidatorProvider::is_validator_active(caller_validator),
                Error::<T>::UnauthorizedFinalize
            );
            ensure!(
                T::PresenceVerifier::is_presence_verified(request.actor, epoch),
                Error::<T>::PresenceNotVerified
            );
            ensure!(
                request.witness_count >= T::WitnessThreshold::get(),
                Error::<T>::WitnessThresholdNotMet
            );

            let mut binding = Numbers::<T>::get(number_id).ok_or(Error::<T>::NumberNotFound)?;
            if let Some(bound_number) = DeviceNumbers::<T>::get(request.pending_device_id) {
                ensure!(bound_number == number_id, Error::<T>::DeviceAlreadyBound);
            }

            if let Some(previous_device_id) = binding.current_device_id {
                if previous_device_id != request.pending_device_id {
                    DeviceNumbers::<T>::remove(previous_device_id);
                }
            }

            let now = frame_system::Pallet::<T>::block_number();
            let lease_until = now.saturating_add(T::ServiceLeaseBlocks::get());

            binding.current_device_id = Some(request.pending_device_id);
            binding.sim_profile_commitment = Some(request.pending_sim_profile_commitment);
            binding.service_lease_until = Some(lease_until);
            binding.activated_at = binding.activated_at.or(Some(now));
            binding.activation_epoch = Some(epoch);
            binding.provisioning_state = ProvisioningState::Pending;
            binding.provisioning_receipt = None;

            match request.kind {
                ServiceRequestKind::Activation => {
                    binding.status = NumberStatus::Activated;
                    Self::deposit_event(Event::ServiceActivated {
                        actor: request.actor,
                        number_id,
                        device_id: request.pending_device_id,
                        epoch,
                        lease_until,
                    });
                }
                ServiceRequestKind::Recovery => {
                    binding.status = NumberStatus::Recovered;
                    Self::deposit_event(Event::ServiceRecovered {
                        actor: request.actor,
                        number_id,
                        device_id: request.pending_device_id,
                        epoch,
                        lease_until,
                    });
                }
            }

            Self::deposit_event(Event::ProvisioningAuthorized {
                actor: request.actor,
                number_id,
                device_id: request.pending_device_id,
                epoch,
            });

            // Aggregate signal quality from witnesses and pay rewards
            Self::aggregate_signal_and_reward(epoch, number_id, request.region_id, &who, now);

            DeviceNumbers::<T>::insert(request.pending_device_id, number_id);
            Numbers::<T>::insert(number_id, binding);
            ServiceRequests::<T>::remove(epoch, number_id);
            CurrentRequestEpoch::<T>::remove(number_id);
            ServiceWitnessCount::<T>::remove(epoch, number_id);
            let _ = ServiceWitnesses::<T>::clear_prefix((epoch, number_id), u32::MAX, None);

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::suspend_number())]
        pub fn suspend_number(
            origin: OriginFor<T>,
            number_id: H256,
            reason: H256,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);

            Numbers::<T>::try_mutate(number_id, |binding| -> DispatchResult {
                let record = binding.as_mut().ok_or(Error::<T>::NumberNotFound)?;
                ensure!(record.owner == actor, Error::<T>::NotNumberOwner);
                ensure!(
                    matches!(
                        record.status,
                        NumberStatus::Activated | NumberStatus::Recovered
                    ),
                    Error::<T>::InvalidNumberStatus
                );
                record.status = NumberStatus::Suspended;
                record.service_lease_until = None;
                Ok(())
            })?;

            Self::deposit_event(Event::NumberSuspended {
                actor,
                number_id,
                reason,
            });
            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::revoke_number())]
        pub fn revoke_number(origin: OriginFor<T>, number_id: H256) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);

            let mut binding = Numbers::<T>::get(number_id).ok_or(Error::<T>::NumberNotFound)?;
            ensure!(binding.owner == actor, Error::<T>::NotNumberOwner);
            ensure!(
                binding.status != NumberStatus::Revoked,
                Error::<T>::NumberRevoked
            );

            if let Some(device_id) = binding.current_device_id {
                DeviceNumbers::<T>::remove(device_id);
            }

            binding.status = NumberStatus::Revoked;
            binding.current_device_id = None;
            binding.service_lease_until = None;
            binding.provisioning_state = ProvisioningState::None;
            binding.provisioning_receipt = None;
            Numbers::<T>::insert(number_id, binding);

            ActorNumbers::<T>::remove(actor, number_id);
            ActorNumberCount::<T>::mutate(actor, |count| *count = count.saturating_sub(1));
            CurrentRequestEpoch::<T>::remove(number_id);

            Self::deposit_event(Event::NumberRevoked { actor, number_id });
            Ok(())
        }

        /// Re-aggregate signal quality from current witnesses during
        /// an active lease. Only callable by the number owner or an
        /// active validator.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::refresh_signal_quality())]
        pub fn refresh_signal_quality(
            origin: OriginFor<T>,
            number_id: H256,
            epoch: EpochId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let caller_actor = Self::account_to_actor(&who);
            let caller_validator = Self::account_to_validator(&who);

            let binding = Numbers::<T>::get(number_id).ok_or(Error::<T>::NumberNotFound)?;
            ensure!(
                caller_actor == binding.owner
                    || T::ValidatorProvider::is_validator_active(caller_validator),
                Error::<T>::UnauthorizedFinalize
            );
            ensure!(
                matches!(
                    binding.status,
                    NumberStatus::Activated | NumberStatus::Recovered
                ),
                Error::<T>::NumberNotActive
            );

            let now = frame_system::Pallet::<T>::block_number();
            Self::compute_signal_aggregation(epoch, number_id, now);

            Ok(())
        }

        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::record_provisioning_result())]
        pub fn record_provisioning_result(
            origin: OriginFor<T>,
            number_id: H256,
            receipt_hash: H256,
            success: bool,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                who == T::CarrierBridgeAccount::get(),
                Error::<T>::ProvisioningUpdateNotAllowed
            );

            Numbers::<T>::try_mutate(number_id, |binding| -> DispatchResult {
                let record = binding.as_mut().ok_or(Error::<T>::NumberNotFound)?;
                ensure!(
                    matches!(
                        record.status,
                        NumberStatus::Activated | NumberStatus::Recovered | NumberStatus::Suspended
                    ),
                    Error::<T>::InvalidNumberStatus
                );

                if success {
                    record.provisioning_state = ProvisioningState::Provisioned;
                    record.provisioning_receipt = Some(receipt_hash);
                    Self::deposit_event(Event::ProvisioningRecorded {
                        number_id,
                        receipt: receipt_hash,
                    });
                } else {
                    record.provisioning_state = ProvisioningState::Failed;
                    record.provisioning_receipt = if receipt_hash == H256::zero() {
                        None
                    } else {
                        Some(receipt_hash)
                    };
                    Self::deposit_event(Event::ProvisioningFailed {
                        number_id,
                        receipt: record.provisioning_receipt,
                    });
                }

                Ok(())
            })
        }

        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::register_service_node())]
        pub fn register_service_node(
            origin: OriginFor<T>,
            regions: BoundedVec<H256, T::MaxRegionsPerServiceNode>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let validator = Self::account_to_validator(&who);

            ensure!(!regions.is_empty(), Error::<T>::NoServiceRegions);
            ensure!(
                T::ValidatorProvider::is_validator_active(validator),
                Error::<T>::ValidatorNotActive
            );
            ensure!(
                !ServiceNodes::<T>::contains_key(validator),
                Error::<T>::ServiceNodeAlreadyRegistered
            );

            let stake_weight = T::ValidatorStakeProvider::validator_stake(validator);
            ensure!(
                stake_weight >= T::MinServiceStake::get(),
                Error::<T>::InsufficientServiceStake
            );

            let now = frame_system::Pallet::<T>::block_number();
            let profile = ServiceNodeProfile::<T> {
                controller: who.clone(),
                validator,
                regions: regions.clone(),
                status: ServiceNodeStatus::Active,
                stake_weight,
                last_presence_epoch: None,
                last_signal_score: None,
                delivery_success_count: 0,
                delivery_failure_count: 0,
                slash_points: 0,
                updated_at: now,
            };
            ServiceNodes::<T>::insert(validator, profile);
            Self::write_service_regions(validator, &regions);

            Self::deposit_event(Event::ServiceNodeRegistered {
                validator,
                region_count: regions.len() as u32,
                stake_weight,
            });
            Ok(())
        }

        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::update_service_regions())]
        pub fn update_service_regions(
            origin: OriginFor<T>,
            regions: BoundedVec<H256, T::MaxRegionsPerServiceNode>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let validator = Self::account_to_validator(&who);

            ensure!(!regions.is_empty(), Error::<T>::NoServiceRegions);

            let mut profile =
                ServiceNodes::<T>::get(validator).ok_or(Error::<T>::ServiceNodeNotFound)?;
            ensure!(
                profile.controller == who,
                Error::<T>::ServiceNodeNotEligible
            );

            Self::clear_service_regions(validator, &profile.regions);
            profile.regions = regions.clone();
            profile.updated_at = frame_system::Pallet::<T>::block_number();
            ServiceNodes::<T>::insert(validator, profile);
            Self::write_service_regions(validator, &regions);

            Self::deposit_event(Event::ServiceNodeRegionsUpdated {
                validator,
                region_count: regions.len() as u32,
            });
            Ok(())
        }

        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::suspend_service_node())]
        pub fn suspend_service_node(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let validator = Self::account_to_validator(&who);

            ServiceNodes::<T>::try_mutate(validator, |profile| -> DispatchResult {
                let profile = profile.as_mut().ok_or(Error::<T>::ServiceNodeNotFound)?;
                ensure!(
                    profile.controller == who,
                    Error::<T>::ServiceNodeNotEligible
                );
                profile.status = ServiceNodeStatus::Suspended;
                profile.updated_at = frame_system::Pallet::<T>::block_number();
                Ok(())
            })?;

            let slash_points = ServiceNodes::<T>::get(validator)
                .map(|profile| profile.slash_points)
                .unwrap_or_default();
            Self::deposit_event(Event::ServiceNodeSuspended {
                validator,
                slash_points,
            });
            Ok(())
        }

        #[pallet::call_index(12)]
        #[pallet::weight(T::WeightInfo::reactivate_service_node())]
        pub fn reactivate_service_node(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let validator = Self::account_to_validator(&who);
            let stake_weight = T::ValidatorStakeProvider::validator_stake(validator);

            ensure!(
                T::ValidatorProvider::is_validator_active(validator),
                Error::<T>::ValidatorNotActive
            );
            ensure!(
                stake_weight >= T::MinServiceStake::get(),
                Error::<T>::InsufficientServiceStake
            );

            ServiceNodes::<T>::try_mutate(validator, |profile| -> DispatchResult {
                let profile = profile.as_mut().ok_or(Error::<T>::ServiceNodeNotFound)?;
                ensure!(
                    profile.controller == who,
                    Error::<T>::ServiceNodeNotEligible
                );
                profile.status = ServiceNodeStatus::Active;
                profile.stake_weight = stake_weight;
                profile.updated_at = frame_system::Pallet::<T>::block_number();
                Ok(())
            })?;

            Self::deposit_event(Event::ServiceNodeReactivated { validator });
            Ok(())
        }

        #[pallet::call_index(13)]
        #[pallet::weight(T::WeightInfo::refresh_region_coverage())]
        pub fn refresh_region_coverage(
            origin: OriginFor<T>,
            region_id: H256,
            epoch: EpochId,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let now = frame_system::Pallet::<T>::block_number();
            Self::refresh_region_coverage_for(region_id, epoch, now);
            Ok(())
        }

        #[pallet::call_index(14)]
        #[pallet::weight(T::WeightInfo::report_service_failure())]
        pub fn report_service_failure(
            origin: OriginFor<T>,
            number_id: H256,
            validator: ValidatorId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let actor = Self::account_to_actor(&who);
            let binding = Numbers::<T>::get(number_id).ok_or(Error::<T>::NumberNotFound)?;

            ensure!(binding.owner == actor, Error::<T>::NotNumberOwner);
            ensure!(
                matches!(
                    binding.status,
                    NumberStatus::Activated | NumberStatus::Recovered | NumberStatus::Suspended
                ),
                Error::<T>::NumberNotActive
            );

            let epoch = binding
                .activation_epoch
                .ok_or(Error::<T>::ServiceRequestNotFound)?;
            ensure!(
                !FailureReports::<T>::contains_key((epoch, number_id, validator)),
                Error::<T>::DuplicateFailureReport
            );

            let mut profile =
                ServiceNodes::<T>::get(validator).ok_or(Error::<T>::ServiceNodeNotFound)?;
            ensure!(
                profile
                    .regions
                    .iter()
                    .any(|region| *region == binding.region_id),
                Error::<T>::ServiceNodeRegionNotServed
            );

            let now = frame_system::Pallet::<T>::block_number();
            profile.delivery_failure_count = profile.delivery_failure_count.saturating_add(1);
            profile.slash_points = profile.slash_points.saturating_add(1);
            profile.updated_at = now;
            let slash_points = profile.slash_points;
            let suspended = slash_points >= T::ServiceNodeSlashThreshold::get();
            if suspended {
                profile.status = ServiceNodeStatus::Suspended;
            }
            ServiceNodes::<T>::insert(validator, profile);
            FailureReports::<T>::insert((epoch, number_id, validator), ());
            Self::refresh_region_coverage_for(binding.region_id, epoch, now);

            Self::deposit_event(Event::ServiceNodeFailureReported {
                validator,
                number_id,
                slash_points,
                suspended,
            });
            if suspended {
                Self::deposit_event(Event::ServiceNodeSuspended {
                    validator,
                    slash_points,
                });
            }
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn account_to_actor(account: &T::AccountId) -> ActorId {
            seveny_primitives::crypto::derive_actor_id(&account.encode())
        }

        fn account_to_validator(account: &T::AccountId) -> ValidatorId {
            seveny_primitives::crypto::derive_validator_id(&account.encode())
        }

        fn ensure_actor_service_ready(
            actor: ActorId,
            device_id: u64,
            epoch: EpochId,
        ) -> DispatchResult {
            ensure!(
                T::ActorChecker::is_actor_active(actor),
                Error::<T>::ActorNotActive
            );
            ensure!(
                T::EpochProvider::is_epoch_active(epoch),
                Error::<T>::EpochNotActive
            );
            ensure!(
                T::DeviceChecker::is_device_active_for_actor(actor, device_id),
                Error::<T>::DeviceNotEligible
            );
            ensure!(
                T::PresenceVerifier::is_presence_verified(actor, epoch),
                Error::<T>::PresenceNotVerified
            );
            Ok(())
        }

        fn ensure_service_node_eligible(
            validator: ValidatorId,
            controller: &T::AccountId,
            region_id: H256,
            epoch: EpochId,
        ) -> DispatchResult {
            let profile =
                ServiceNodes::<T>::get(validator).ok_or(Error::<T>::ServiceNodeNotFound)?;
            ensure!(
                profile.controller == *controller,
                Error::<T>::ServiceNodeNotEligible
            );
            ensure!(
                profile.regions.iter().any(|region| *region == region_id),
                Error::<T>::ServiceNodeRegionNotServed
            );
            ensure!(
                profile.status == ServiceNodeStatus::Active,
                Error::<T>::ServiceNodeNotEligible
            );
            ensure!(
                T::ValidatorProvider::is_validator_active(validator),
                Error::<T>::ValidatorNotActive
            );
            ensure!(
                T::ValidatorStakeProvider::validator_stake(validator) >= T::MinServiceStake::get(),
                Error::<T>::InsufficientServiceStake
            );
            ensure!(
                T::ServiceNodePresenceVerifier::is_service_node_present(controller, epoch),
                Error::<T>::ServiceNodeNotEligible
            );
            Ok(())
        }

        fn write_service_regions(
            validator: ValidatorId,
            regions: &BoundedVec<H256, T::MaxRegionsPerServiceNode>,
        ) {
            for region_id in regions.iter() {
                RegionServiceNodes::<T>::insert(region_id, validator, ());
            }
        }

        fn clear_service_regions(
            validator: ValidatorId,
            regions: &BoundedVec<H256, T::MaxRegionsPerServiceNode>,
        ) {
            for region_id in regions.iter() {
                RegionServiceNodes::<T>::remove(region_id, validator);
            }
        }

        fn coverage_status_for(
            eligible_node_count: u32,
            avg_signal_score: Option<u8>,
        ) -> CoverageStatus {
            if eligible_node_count == 0 {
                return CoverageStatus::Unavailable;
            }
            if eligible_node_count < T::MinRegionalServiceNodes::get() {
                return CoverageStatus::Weak;
            }
            if eligible_node_count >= T::StrongRegionalServiceNodes::get()
                && avg_signal_score.unwrap_or(0) >= 80
            {
                return CoverageStatus::Strong;
            }

            CoverageStatus::Available
        }

        fn refresh_region_coverage_for(
            region_id: H256,
            epoch: EpochId,
            now: BlockNumberFor<T>,
        ) -> RegionCoverageSummary<T> {
            let mut registered_node_count: u32 = 0;
            let mut eligible_node_count: u32 = 0;
            let mut signal_sum: u32 = 0;
            let mut signal_samples: u32 = 0;

            for (validator, _) in RegionServiceNodes::<T>::iter_prefix(region_id) {
                registered_node_count = registered_node_count.saturating_add(1);

                if let Some(mut profile) = ServiceNodes::<T>::get(validator) {
                    let stake_weight = T::ValidatorStakeProvider::validator_stake(validator);
                    let present = T::ServiceNodePresenceVerifier::is_service_node_present(
                        &profile.controller,
                        epoch,
                    );
                    profile.stake_weight = stake_weight;
                    profile.last_presence_epoch = if present { Some(epoch) } else { None };
                    profile.updated_at = now;

                    let eligible = profile.status == ServiceNodeStatus::Active
                        && T::ValidatorProvider::is_validator_active(validator)
                        && stake_weight >= T::MinServiceStake::get()
                        && present
                        && profile
                            .regions
                            .iter()
                            .any(|candidate| *candidate == region_id);
                    if eligible {
                        eligible_node_count = eligible_node_count.saturating_add(1);
                        if let Some(signal_score) = profile.last_signal_score {
                            signal_sum = signal_sum.saturating_add(signal_score as u32);
                            signal_samples = signal_samples.saturating_add(1);
                        }
                    }

                    ServiceNodes::<T>::insert(validator, profile);
                }
            }

            let avg_signal_score = if signal_samples > 0 {
                Some((signal_sum / signal_samples) as u8)
            } else {
                None
            };
            let status = Self::coverage_status_for(eligible_node_count, avg_signal_score);
            Self::deposit_event(Event::RegionCoverageUpdated {
                epoch,
                region_id,
                eligible_node_count,
                status,
                avg_signal_score,
            });

            RegionCoverage::<T>::insert(
                epoch,
                region_id,
                RegionCoverageSummary::<T> {
                    epoch,
                    region_id,
                    registered_node_count,
                    eligible_node_count,
                    avg_signal_score,
                    status,
                    updated_at: now,
                },
            );

            RegionCoverageSummary::<T> {
                epoch,
                region_id,
                registered_node_count,
                eligible_node_count,
                avg_signal_score,
                status,
                updated_at: now,
            }
        }

        /// Iterate witnesses for (epoch, number_id), compute signal
        /// aggregation, pay rewards, and update leaderboard.
        fn aggregate_signal_and_reward(
            epoch: EpochId,
            number_id: H256,
            region_id: H256,
            _caller: &T::AccountId,
            now: BlockNumberFor<T>,
        ) {
            let mut scores: alloc::vec::Vec<u8> = alloc::vec::Vec::new();
            let mut witness_entries: alloc::vec::Vec<(ValidatorId, u8)> = alloc::vec::Vec::new();

            for (validator_key, witness) in ServiceWitnesses::<T>::iter_prefix((epoch, number_id)) {
                scores.push(witness.signal_score);
                witness_entries.push((validator_key, witness.signal_score));
            }

            // Signal quality aggregation
            if !scores.is_empty() {
                let count = scores.len() as u32;
                let sum: u32 = scores.iter().map(|s| *s as u32).sum();
                let avg = (sum / count) as u8;
                let min = scores.iter().copied().min().unwrap_or(0);
                let max = scores.iter().copied().max().unwrap_or(0);

                let aggregation = SignalAggregation::<T> {
                    avg_score: avg,
                    min_score: min,
                    max_score: max,
                    sample_count: count,
                    last_updated: now,
                };
                SignalQuality::<T>::insert(number_id, aggregation);

                Self::deposit_event(Event::SignalQualityUpdated {
                    number_id,
                    avg_score: avg,
                    sample_count: count,
                });
            }

            // Witness rewards
            let reward_amount = T::WitnessRewardAmount::get();
            for (validator, signal_score) in &witness_entries {
                ServiceNodes::<T>::mutate(validator, |profile| {
                    if let Some(profile) = profile {
                        profile.delivery_success_count =
                            profile.delivery_success_count.saturating_add(1);
                        profile.last_signal_score = Some(*signal_score);
                        profile.last_presence_epoch = Some(epoch);
                        profile.stake_weight =
                            T::ValidatorStakeProvider::validator_stake(*validator);
                        profile.updated_at = now;
                        Self::deposit_event(Event::ServiceNodeDeliveryRecorded {
                            validator: *validator,
                            number_id,
                            signal_score: *signal_score,
                            success_count: profile.delivery_success_count,
                        });
                    }
                });

                if reward_amount > 0 {
                    T::RewardHandler::reward_witness(validator, reward_amount);
                    ValidatorWitnessCount::<T>::mutate(validator, |c| {
                        *c = c.saturating_add(1);
                    });
                    Self::deposit_event(Event::WitnessRewardPaid {
                        validator: *validator,
                        number_id,
                        amount: reward_amount,
                    });
                }
            }

            Self::refresh_region_coverage_for(region_id, epoch, now);
        }

        /// Compute signal aggregation without rewards (for refresh).
        fn compute_signal_aggregation(epoch: EpochId, number_id: H256, now: BlockNumberFor<T>) {
            let mut scores: alloc::vec::Vec<u8> = alloc::vec::Vec::new();

            for (_validator_key, witness) in ServiceWitnesses::<T>::iter_prefix((epoch, number_id))
            {
                scores.push(witness.signal_score);
            }

            if !scores.is_empty() {
                let count = scores.len() as u32;
                let sum: u32 = scores.iter().map(|s| *s as u32).sum();
                let avg = (sum / count) as u8;
                let min = scores.iter().copied().min().unwrap_or(0);
                let max = scores.iter().copied().max().unwrap_or(0);

                let aggregation = SignalAggregation::<T> {
                    avg_score: avg,
                    min_score: min,
                    max_score: max,
                    sample_count: count,
                    last_updated: now,
                };
                SignalQuality::<T>::insert(number_id, aggregation);

                Self::deposit_event(Event::SignalQualityUpdated {
                    number_id,
                    avg_score: avg,
                    sample_count: count,
                });
            }
        }
    }
}
