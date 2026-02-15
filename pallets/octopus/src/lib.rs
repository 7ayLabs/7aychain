#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;
pub mod fusion;
pub mod weights;

pub use fusion::{
    FusedHealthMetrics, FusionWeights, HealingAction, HealingTrigger,
    Position as FusionPosition,
};

#[cfg(test)]
mod tests;

use alloc::vec::Vec;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use seveny_primitives::types::ActorId;
use sp_arithmetic::Perbill;
use sp_runtime::Saturating;

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Default,
    Hash,
)]
pub struct SubnodeId(pub u64);

impl SubnodeId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
    Default,
    Hash,
)]
pub struct ClusterId(pub u64);

impl ClusterId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum SubnodeStatus {
    Inactive,
    Activating,
    Active,
    Deactivating,
    Failed,
}

impl Default for SubnodeStatus {
    fn default() -> Self {
        Self::Inactive
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum ScalingDecision {
    Maintain,
    ScaleUp(u32),
    ScaleDown,
}

impl Default for ScalingDecision {
    fn default() -> Self {
        Self::Maintain
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum ClusterStatus {
    Initializing,
    Running,
    Scaling,
    Degraded,
    Shutdown,
}

impl Default for ClusterStatus {
    fn default() -> Self {
        Self::Initializing
    }
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
pub struct Subnode<T: Config> {
    pub id: SubnodeId,
    pub cluster: ClusterId,
    pub operator: ActorId,
    pub status: SubnodeStatus,
    pub throughput: Perbill,
    pub created_at: BlockNumberFor<T>,
    pub activated_at: Option<BlockNumberFor<T>>,
    pub deactivation_started: Option<BlockNumberFor<T>>,
    pub processed_count: u64,
    pub last_heartbeat: BlockNumberFor<T>,
    pub consecutive_misses: u8,
    pub health_score: u8,
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
pub struct Cluster<T: Config> {
    pub id: ClusterId,
    pub owner: ActorId,
    pub status: ClusterStatus,
    pub active_subnodes: u32,
    pub max_subnodes: u32,
    pub total_throughput: Perbill,
    pub created_at: BlockNumberFor<T>,
    pub last_scaling_at: BlockNumberFor<T>,
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
pub struct ThroughputMetric<T: Config> {
    pub cluster: ClusterId,
    pub throughput: Perbill,
    pub recorded_at: BlockNumberFor<T>,
    pub sample_count: u32,
}

/// Diagnostic action to remediate subnode issues
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum DiagnosticAction {
    /// Restart heartbeat monitoring
    RestartHeartbeat,
    /// Reset fused health metrics
    ResetFusedHealth,
    /// Re-register with cluster
    ReregisterCluster,
    /// Recalibrate position
    RecalibratePosition,
    /// Rotate authentication profile
    RotateAuthProfile,
    /// Clear device cache
    ClearDeviceCache,
    /// Escalate to operator
    EscalateOperator,
}

/// Severity level of diagnostic report
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    parity_scale_codec::DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum DiagnosticSeverity {
    /// Everything is healthy
    Healthy,
    /// Minor issues detected
    Warning,
    /// Significant issues requiring attention
    Critical,
    /// Node has failed
    Failed,
}

impl Default for DiagnosticSeverity {
    fn default() -> Self {
        Self::Healthy
    }
}

/// Health checks performed by diagnostics
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
    Default,
)]
pub struct DiagnosticChecks {
    /// Whether heartbeat is functioning
    pub heartbeat_ok: bool,
    /// Whether device observations are being recorded
    pub device_observations_ok: bool,
    /// Whether position is consistent
    pub position_consistency_ok: bool,
    /// Whether cluster connectivity is ok
    pub cluster_connectivity_ok: bool,
    /// Fused health score
    pub fused_health_score: u8,
}

/// Maximum number of diagnostic actions
pub type MaxDiagnosticActions = ConstU32<10>;

/// Diagnostic report for a subnode
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
pub struct DiagnosticReport<BlockNumber> {
    /// The subnode being diagnosed
    pub subnode_id: SubnodeId,
    /// Health checks performed
    pub checks: DiagnosticChecks,
    /// Recommended remediation actions
    pub actions: BoundedVec<DiagnosticAction, MaxDiagnosticActions>,
    /// Severity of the diagnosis
    pub severity: DiagnosticSeverity,
    /// When the report was generated
    pub generated_at: BlockNumber,
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
        type ActivationThreshold: Get<Perbill>;

        #[pallet::constant]
        type DeactivationThreshold: Get<Perbill>;

        #[pallet::constant]
        type DeactivationDurationBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MaxSubnodesPerCluster: Get<u32>;

        #[pallet::constant]
        type MinSubnodes: Get<u32>;

        #[pallet::constant]
        type ScalingCooldownBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type HeartbeatTimeoutBlocks: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type MaxConsecutiveMisses: Get<u8>;

        #[pallet::constant]
        type HealthScoreDecay: Get<u8>;

        #[pallet::constant]
        type HealthScoreRecovery: Get<u8>;
    }

    #[pallet::storage]
    #[pallet::getter(fn subnode_count)]
    pub type SubnodeCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn cluster_count)]
    pub type ClusterCount<T> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn subnodes)]
    pub type Subnodes<T: Config> = StorageMap<_, Blake2_128Concat, SubnodeId, Subnode<T>>;

    #[pallet::storage]
    #[pallet::getter(fn clusters)]
    pub type Clusters<T: Config> = StorageMap<_, Blake2_128Concat, ClusterId, Cluster<T>>;

    #[pallet::storage]
    #[pallet::getter(fn cluster_subnodes)]
    pub type ClusterSubnodes<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, ClusterId, Blake2_128Concat, SubnodeId, ()>;

    #[pallet::storage]
    #[pallet::getter(fn throughput_history)]
    pub type ThroughputHistory<T: Config> =
        StorageMap<_, Blake2_128Concat, ClusterId, ThroughputMetric<T>>;

    #[pallet::storage]
    #[pallet::getter(fn operator_subnodes)]
    pub type OperatorSubnodes<T: Config> =
        StorageDoubleMap<_, Blake2_128Concat, ActorId, Blake2_128Concat, SubnodeId, ()>;

    #[pallet::storage]
    #[pallet::getter(fn active_subnode_count)]
    pub type ActiveSubnodeCount<T> = StorageValue<_, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn fused_health)]
    pub type FusedHealth<T: Config> =
        StorageMap<_, Blake2_128Concat, SubnodeId, FusedHealthMetrics>;

    #[pallet::storage]
    #[pallet::getter(fn fusion_weights)]
    pub type GlobalFusionWeights<T> = StorageValue<_, FusionWeights, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        pub _phantom: PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            SubnodeCount::<T>::put(0u64);
            ClusterCount::<T>::put(0u64);
            ActiveSubnodeCount::<T>::put(0u32);
            GlobalFusionWeights::<T>::put(FusionWeights::default_weights());
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        ClusterCreated {
            cluster_id: ClusterId,
            owner: ActorId,
        },
        SubnodeRegistered {
            subnode_id: SubnodeId,
            cluster_id: ClusterId,
            operator: ActorId,
        },
        SubnodeActivated {
            subnode_id: SubnodeId,
            cluster_id: ClusterId,
        },
        SubnodeDeactivationStarted {
            subnode_id: SubnodeId,
            cluster_id: ClusterId,
        },
        SubnodeDeactivated {
            subnode_id: SubnodeId,
            cluster_id: ClusterId,
        },
        ScalingDecisionMade {
            cluster_id: ClusterId,
            decision: ScalingDecision,
            throughput: Perbill,
        },
        ThroughputUpdated {
            cluster_id: ClusterId,
            throughput: Perbill,
        },
        ClusterStatusChanged {
            cluster_id: ClusterId,
            old_status: ClusterStatus,
            new_status: ClusterStatus,
        },
        HeartbeatReceived {
            subnode_id: SubnodeId,
            health_score: u8,
        },
        SubnodeFailed {
            subnode_id: SubnodeId,
            cluster_id: ClusterId,
            consecutive_misses: u8,
        },
        AutoHealingInitiated {
            cluster_id: ClusterId,
            failed_count: u32,
            active_remaining: u32,
        },
        SubnodeHealthUpdated {
            subnode_id: SubnodeId,
            old_score: u8,
            new_score: u8,
        },
        FusedHealthUpdated {
            subnode_id: SubnodeId,
            heartbeat_component: u8,
            device_component: u8,
            position_component: u8,
            fused_score: u8,
        },
        DeviceObservationRecorded {
            subnode_id: SubnodeId,
            device_count: u8,
            commitment: sp_core::H256,
        },
        PositionConfirmed {
            subnode_id: SubnodeId,
            position: FusionPosition,
            variance: u32,
        },
        FusionHealingTriggered {
            subnode_id: SubnodeId,
            trigger: HealingTrigger,
            previous_score: u8,
        },
        /// Diagnostic report generated
        DiagnosticReportGenerated {
            subnode_id: SubnodeId,
            severity: DiagnosticSeverity,
            actions_count: u32,
        },
        /// Auto-fix applied to subnode
        AutoFixApplied {
            subnode_id: SubnodeId,
            actions_applied: u32,
        },
        /// Inactive subnode was pruned
        SubnodePruned {
            subnode_id: SubnodeId,
        },
        /// Operator escalation required
        OperatorEscalationRequired {
            subnode_id: SubnodeId,
            reason: DiagnosticSeverity,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        ClusterNotFound,
        SubnodeNotFound,
        MaxSubnodesReached,
        SubnodeAlreadyActive,
        SubnodeNotActive,
        InsufficientThroughput,
        ScalingCooldownActive,
        ClusterNotRunning,
        NotClusterOwner,
        NotSubnodeOperator,
        InvalidThroughput,
        MinSubnodesRequired,
        SubnodeAlreadyDeactivating,
        DeactivationNotComplete,
        SubnodeFailed,
        HeartbeatTooFrequent,
        InvalidCommitment,
        NoFusedHealthRecord,
        InvalidFusionWeights,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            Self::process_deactivations(n);
            Self::detect_failed_nodes(n);
            Self::auto_heal_clusters(n);
            T::DbWeight::get().reads(1)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_cluster())]
        pub fn create_cluster(origin: OriginFor<T>, owner: ActorId) -> DispatchResult {
            ensure_signed(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();
            let cluster_id = Self::next_cluster_id();

            let cluster = Cluster {
                id: cluster_id,
                owner,
                status: ClusterStatus::Initializing,
                active_subnodes: 0,
                max_subnodes: T::MaxSubnodesPerCluster::get(),
                total_throughput: Perbill::zero(),
                created_at: block_number,
                last_scaling_at: block_number,
            };

            Clusters::<T>::insert(cluster_id, cluster);

            Self::deposit_event(Event::ClusterCreated { cluster_id, owner });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::register_subnode())]
        pub fn register_subnode(
            origin: OriginFor<T>,
            cluster_id: ClusterId,
            operator: ActorId,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let cluster = Clusters::<T>::get(cluster_id).ok_or(Error::<T>::ClusterNotFound)?;

            let subnode_count = ClusterSubnodes::<T>::iter_prefix(cluster_id).count() as u32;
            ensure!(
                subnode_count < cluster.max_subnodes,
                Error::<T>::MaxSubnodesReached
            );

            let block_number = frame_system::Pallet::<T>::block_number();
            let subnode_id = Self::next_subnode_id();

            let subnode = Subnode {
                id: subnode_id,
                cluster: cluster_id,
                operator,
                status: SubnodeStatus::Inactive,
                throughput: Perbill::zero(),
                created_at: block_number,
                activated_at: None,
                deactivation_started: None,
                processed_count: 0,
                last_heartbeat: block_number,
                consecutive_misses: 0,
                health_score: 100,
            };

            Subnodes::<T>::insert(subnode_id, subnode);
            ClusterSubnodes::<T>::insert(cluster_id, subnode_id, ());
            OperatorSubnodes::<T>::insert(operator, subnode_id, ());

            Self::deposit_event(Event::SubnodeRegistered {
                subnode_id,
                cluster_id,
                operator,
            });

            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::activate_subnode())]
        pub fn activate_subnode(origin: OriginFor<T>, subnode_id: SubnodeId) -> DispatchResult {
            ensure_signed(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();

            Subnodes::<T>::try_mutate(subnode_id, |subnode| -> DispatchResult {
                let s = subnode.as_mut().ok_or(Error::<T>::SubnodeNotFound)?;

                ensure!(
                    s.status == SubnodeStatus::Inactive,
                    Error::<T>::SubnodeAlreadyActive
                );

                s.status = SubnodeStatus::Active;
                s.activated_at = Some(block_number);

                let cluster_id = s.cluster;

                Clusters::<T>::mutate(cluster_id, |cluster| {
                    if let Some(ref mut c) = cluster {
                        c.active_subnodes = c.active_subnodes.saturating_add(1);
                        if c.status == ClusterStatus::Initializing {
                            c.status = ClusterStatus::Running;
                            Self::deposit_event(Event::ClusterStatusChanged {
                                cluster_id,
                                old_status: ClusterStatus::Initializing,
                                new_status: ClusterStatus::Running,
                            });
                        }
                    }
                });

                ActiveSubnodeCount::<T>::mutate(|count| *count = count.saturating_add(1));

                Self::deposit_event(Event::SubnodeActivated {
                    subnode_id,
                    cluster_id,
                });

                Ok(())
            })
        }

        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::start_deactivation())]
        pub fn start_deactivation(origin: OriginFor<T>, subnode_id: SubnodeId) -> DispatchResult {
            ensure_signed(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();

            Subnodes::<T>::try_mutate(subnode_id, |subnode| -> DispatchResult {
                let s = subnode.as_mut().ok_or(Error::<T>::SubnodeNotFound)?;

                ensure!(
                    s.status == SubnodeStatus::Active,
                    Error::<T>::SubnodeNotActive
                );

                let cluster_id = s.cluster;
                let cluster = Clusters::<T>::get(cluster_id).ok_or(Error::<T>::ClusterNotFound)?;

                ensure!(
                    cluster.active_subnodes > T::MinSubnodes::get(),
                    Error::<T>::MinSubnodesRequired
                );

                s.status = SubnodeStatus::Deactivating;
                s.deactivation_started = Some(block_number);

                Self::deposit_event(Event::SubnodeDeactivationStarted {
                    subnode_id,
                    cluster_id,
                });

                Ok(())
            })
        }

        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::update_throughput())]
        pub fn update_throughput(
            origin: OriginFor<T>,
            cluster_id: ClusterId,
            throughput: Perbill,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();

            Clusters::<T>::try_mutate(cluster_id, |cluster| -> DispatchResult {
                let c = cluster.as_mut().ok_or(Error::<T>::ClusterNotFound)?;
                c.total_throughput = throughput;
                Ok(())
            })?;

            let metric = ThroughputMetric {
                cluster: cluster_id,
                throughput,
                recorded_at: block_number,
                sample_count: ThroughputHistory::<T>::get(cluster_id)
                    .map(|m| m.sample_count.saturating_add(1))
                    .unwrap_or(1),
            };

            ThroughputHistory::<T>::insert(cluster_id, metric);

            Self::deposit_event(Event::ThroughputUpdated {
                cluster_id,
                throughput,
            });

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::evaluate_scaling())]
        pub fn evaluate_scaling(origin: OriginFor<T>, cluster_id: ClusterId) -> DispatchResult {
            ensure_signed(origin)?;

            let cluster = Clusters::<T>::get(cluster_id).ok_or(Error::<T>::ClusterNotFound)?;
            let block_number = frame_system::Pallet::<T>::block_number();

            let cooldown_elapsed = block_number.saturating_sub(cluster.last_scaling_at)
                >= T::ScalingCooldownBlocks::get();

            ensure!(cooldown_elapsed, Error::<T>::ScalingCooldownActive);

            let throughput = cluster.total_throughput;
            let decision = Self::compute_scaling_decision(throughput, cluster.active_subnodes);

            if decision != ScalingDecision::Maintain {
                Clusters::<T>::mutate(cluster_id, |c| {
                    if let Some(ref mut cluster) = c {
                        cluster.last_scaling_at = block_number;
                        cluster.status = ClusterStatus::Scaling;
                    }
                });
            }

            Self::deposit_event(Event::ScalingDecisionMade {
                cluster_id,
                decision,
                throughput,
            });

            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::update_subnode_throughput())]
        pub fn update_subnode_throughput(
            origin: OriginFor<T>,
            subnode_id: SubnodeId,
            throughput: Perbill,
            processed: u64,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            Subnodes::<T>::try_mutate(subnode_id, |subnode| -> DispatchResult {
                let s = subnode.as_mut().ok_or(Error::<T>::SubnodeNotFound)?;
                s.throughput = throughput;
                s.processed_count = s.processed_count.saturating_add(processed);
                Ok(())
            })
        }

        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::activate_subnode())]
        pub fn record_heartbeat(origin: OriginFor<T>, subnode_id: SubnodeId) -> DispatchResult {
            ensure_signed(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();

            Subnodes::<T>::try_mutate(subnode_id, |subnode| -> DispatchResult {
                let s = subnode.as_mut().ok_or(Error::<T>::SubnodeNotFound)?;

                ensure!(
                    s.status == SubnodeStatus::Active,
                    Error::<T>::SubnodeNotActive
                );

                let old_score = s.health_score;
                s.last_heartbeat = block_number;
                s.consecutive_misses = 0;
                s.health_score = old_score.saturating_add(T::HealthScoreRecovery::get()).min(100);

                Self::deposit_event(Event::HeartbeatReceived {
                    subnode_id,
                    health_score: s.health_score,
                });

                if old_score != s.health_score {
                    Self::deposit_event(Event::SubnodeHealthUpdated {
                        subnode_id,
                        old_score,
                        new_score: s.health_score,
                    });
                }

                Ok(())
            })
        }

        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::activate_subnode())]
        pub fn record_device_observation(
            origin: OriginFor<T>,
            subnode_id: SubnodeId,
            device_count: u8,
            commitment: sp_core::H256,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();
            let block_u64: u64 = block_number
                .try_into()
                .map_err(|_| Error::<T>::SubnodeNotFound)?;

            Subnodes::<T>::get(subnode_id).ok_or(Error::<T>::SubnodeNotFound)?;

            let weights = GlobalFusionWeights::<T>::get();

            FusedHealth::<T>::mutate(subnode_id, |maybe_health| {
                let health = maybe_health.get_or_insert_with(|| {
                    FusedHealthMetrics::new(FusionPosition::default())
                });

                health.record_device_observation(device_count, block_u64, commitment, &weights);

                Self::deposit_event(Event::DeviceObservationRecorded {
                    subnode_id,
                    device_count,
                    commitment,
                });

                Self::deposit_event(Event::FusedHealthUpdated {
                    subnode_id,
                    heartbeat_component: health.heartbeat_score,
                    device_component: health.device_metrics.device_score(),
                    position_component: health.position_metrics.position_score(),
                    fused_score: health.fused_score,
                });
            });

            Ok(())
        }

        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::activate_subnode())]
        pub fn record_position_confirmation(
            origin: OriginFor<T>,
            subnode_id: SubnodeId,
            position_x: i64,
            position_y: i64,
            position_z: i64,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();
            let block_u64: u64 = block_number
                .try_into()
                .map_err(|_| Error::<T>::SubnodeNotFound)?;

            Subnodes::<T>::get(subnode_id).ok_or(Error::<T>::SubnodeNotFound)?;

            let position = FusionPosition::new(position_x, position_y, position_z);
            let weights = GlobalFusionWeights::<T>::get();

            FusedHealth::<T>::mutate(subnode_id, |maybe_health| {
                let health = maybe_health.get_or_insert_with(|| {
                    FusedHealthMetrics::new(position.clone())
                });

                health.record_position_confirmation(position.clone(), block_u64, &weights);

                Self::deposit_event(Event::PositionConfirmed {
                    subnode_id,
                    position,
                    variance: health.position_metrics.position_variance,
                });

                Self::deposit_event(Event::FusedHealthUpdated {
                    subnode_id,
                    heartbeat_component: health.heartbeat_score,
                    device_component: health.device_metrics.device_score(),
                    position_component: health.position_metrics.position_score(),
                    fused_score: health.fused_score,
                });
            });

            Ok(())
        }

        #[pallet::call_index(10)]
        #[pallet::weight(T::WeightInfo::activate_subnode())]
        pub fn heartbeat_with_device_proof(
            origin: OriginFor<T>,
            subnode_id: SubnodeId,
            device_count: u8,
            commitment: sp_core::H256,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let block_number = frame_system::Pallet::<T>::block_number();
            let block_u64: u64 = block_number
                .try_into()
                .map_err(|_| Error::<T>::SubnodeNotFound)?;

            Subnodes::<T>::try_mutate(subnode_id, |subnode| -> DispatchResult {
                let s = subnode.as_mut().ok_or(Error::<T>::SubnodeNotFound)?;

                ensure!(
                    s.status == SubnodeStatus::Active,
                    Error::<T>::SubnodeNotActive
                );

                let old_score = s.health_score;
                s.last_heartbeat = block_number;
                s.consecutive_misses = 0;
                s.health_score = old_score.saturating_add(T::HealthScoreRecovery::get()).min(100);

                Self::deposit_event(Event::HeartbeatReceived {
                    subnode_id,
                    health_score: s.health_score,
                });

                Ok(())
            })?;

            let weights = GlobalFusionWeights::<T>::get();

            FusedHealth::<T>::mutate(subnode_id, |maybe_health| {
                let health = maybe_health.get_or_insert_with(|| {
                    FusedHealthMetrics::new(FusionPosition::default())
                });

                let new_heartbeat_score = Subnodes::<T>::get(subnode_id)
                    .map(|s| s.health_score)
                    .unwrap_or(100);

                health.update_heartbeat(new_heartbeat_score, block_u64, &weights);
                health.record_device_observation(device_count, block_u64, commitment, &weights);

                Self::deposit_event(Event::DeviceObservationRecorded {
                    subnode_id,
                    device_count,
                    commitment,
                });

                Self::deposit_event(Event::FusedHealthUpdated {
                    subnode_id,
                    heartbeat_component: health.heartbeat_score,
                    device_component: health.device_metrics.device_score(),
                    position_component: health.position_metrics.position_score(),
                    fused_score: health.fused_score,
                });
            });

            Ok(())
        }

        #[pallet::call_index(11)]
        #[pallet::weight(T::WeightInfo::update_throughput())]
        pub fn set_fusion_weights(
            origin: OriginFor<T>,
            heartbeat_weight: u8,
            device_weight: u8,
            position_weight: u8,
        ) -> DispatchResult {
            ensure_root(origin)?;

            let weights = FusionWeights::new(heartbeat_weight, device_weight, position_weight)
                .ok_or(Error::<T>::InvalidFusionWeights)?;

            GlobalFusionWeights::<T>::put(weights);

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn next_subnode_id() -> SubnodeId {
            let id = SubnodeCount::<T>::get();
            SubnodeCount::<T>::put(id.saturating_add(1));
            SubnodeId::new(id)
        }

        fn next_cluster_id() -> ClusterId {
            let id = ClusterCount::<T>::get();
            ClusterCount::<T>::put(id.saturating_add(1));
            ClusterId::new(id)
        }

        fn compute_scaling_decision(throughput: Perbill, current_subnodes: u32) -> ScalingDecision {
            let activation_threshold = T::ActivationThreshold::get();
            let deactivation_threshold = T::DeactivationThreshold::get();
            let max_subnodes = T::MaxSubnodesPerCluster::get();

            if throughput >= activation_threshold && current_subnodes < max_subnodes {
                let target = Self::calculate_target_subnodes(throughput);
                if target > current_subnodes {
                    return ScalingDecision::ScaleUp(target);
                }
            }

            if throughput <= deactivation_threshold && current_subnodes > T::MinSubnodes::get() {
                return ScalingDecision::ScaleDown;
            }

            ScalingDecision::Maintain
        }

        fn calculate_target_subnodes(throughput: Perbill) -> u32 {
            let pct = throughput.deconstruct() / 10_000_000;
            let scaled = pct.saturating_mul(10);
            let divisor = 225u32;
            let result = scaled.saturating_add(divisor.saturating_sub(1)) / divisor;

            let max = T::MaxSubnodesPerCluster::get();
            let min = T::MinSubnodes::get();

            result.clamp(min, max)
        }

        #[allow(clippy::excessive_nesting)]
        fn process_deactivations(block_number: BlockNumberFor<T>) {
            let duration = T::DeactivationDurationBlocks::get();

            for (subnode_id, mut subnode) in Subnodes::<T>::iter() {
                if subnode.status == SubnodeStatus::Deactivating {
                    if let Some(started) = subnode.deactivation_started {
                        if block_number.saturating_sub(started) >= duration {
                            subnode.status = SubnodeStatus::Inactive;
                            subnode.deactivation_started = None;
                            let cluster_id = subnode.cluster;

                            Subnodes::<T>::insert(subnode_id, subnode);

                            Clusters::<T>::mutate(cluster_id, |cluster| {
                                if let Some(ref mut c) = cluster {
                                    c.active_subnodes = c.active_subnodes.saturating_sub(1);
                                }
                            });

                            ActiveSubnodeCount::<T>::mutate(|count| {
                                *count = count.saturating_sub(1)
                            });

                            Self::deposit_event(Event::SubnodeDeactivated {
                                subnode_id,
                                cluster_id,
                            });
                        }
                    }
                }
            }
        }

        pub fn get_cluster_subnodes(cluster_id: ClusterId) -> Vec<SubnodeId> {
            ClusterSubnodes::<T>::iter_prefix(cluster_id)
                .map(|(subnode_id, _)| subnode_id)
                .collect()
        }

        pub fn get_active_subnodes(cluster_id: ClusterId) -> Vec<SubnodeId> {
            ClusterSubnodes::<T>::iter_prefix(cluster_id)
                .filter_map(|(subnode_id, _)| {
                    Subnodes::<T>::get(subnode_id)
                        .filter(|s| s.status == SubnodeStatus::Active)
                        .map(|_| subnode_id)
                })
                .collect()
        }

        pub fn get_cluster_throughput(cluster_id: ClusterId) -> Perbill {
            Clusters::<T>::get(cluster_id)
                .map(|c| c.total_throughput)
                .unwrap_or(Perbill::zero())
        }

        pub fn is_scaling_needed(cluster_id: ClusterId) -> Option<ScalingDecision> {
            Clusters::<T>::get(cluster_id)
                .map(|c| Self::compute_scaling_decision(c.total_throughput, c.active_subnodes))
        }

        #[allow(clippy::excessive_nesting)]
        fn detect_failed_nodes(block_number: BlockNumberFor<T>) {
            let timeout = T::HeartbeatTimeoutBlocks::get();
            let max_misses = T::MaxConsecutiveMisses::get();
            let decay = T::HealthScoreDecay::get();

            for (subnode_id, mut subnode) in Subnodes::<T>::iter() {
                if subnode.status != SubnodeStatus::Active {
                    continue;
                }

                let blocks_since = block_number.saturating_sub(subnode.last_heartbeat);
                if blocks_since < timeout {
                    continue;
                }

                let old_score = subnode.health_score;
                subnode.consecutive_misses = subnode.consecutive_misses.saturating_add(1);
                subnode.health_score = subnode.health_score.saturating_sub(decay);
                subnode.last_heartbeat = block_number;

                if old_score != subnode.health_score {
                    Self::deposit_event(Event::SubnodeHealthUpdated {
                        subnode_id,
                        old_score,
                        new_score: subnode.health_score,
                    });
                }

                if subnode.consecutive_misses >= max_misses {
                    let cluster_id = subnode.cluster;
                    subnode.status = SubnodeStatus::Failed;

                    Subnodes::<T>::insert(subnode_id, subnode.clone());

                    Clusters::<T>::mutate(cluster_id, |cluster| {
                        if let Some(ref mut c) = cluster {
                            c.active_subnodes = c.active_subnodes.saturating_sub(1);
                            if c.active_subnodes < T::MinSubnodes::get() {
                                c.status = ClusterStatus::Degraded;
                            }
                        }
                    });

                    ActiveSubnodeCount::<T>::mutate(|count| {
                        *count = count.saturating_sub(1)
                    });

                    Self::deposit_event(Event::SubnodeFailed {
                        subnode_id,
                        cluster_id,
                        consecutive_misses: subnode.consecutive_misses,
                    });
                } else {
                    Subnodes::<T>::insert(subnode_id, subnode);
                }
            }
        }

        fn auto_heal_clusters(block_number: BlockNumberFor<T>) {
            let block_u64: u64 = block_number
                .try_into()
                .unwrap_or(0);

            Self::check_fusion_healing_triggers(block_u64);

            for (cluster_id, cluster) in Clusters::<T>::iter() {
                if cluster.status != ClusterStatus::Degraded {
                    continue;
                }

                let min_subnodes = T::MinSubnodes::get();
                if cluster.active_subnodes >= min_subnodes {
                    Clusters::<T>::mutate(cluster_id, |c| {
                        if let Some(ref mut cluster) = c {
                            cluster.status = ClusterStatus::Running;
                        }
                    });
                    continue;
                }

                let failed_count = ClusterSubnodes::<T>::iter_prefix(cluster_id)
                    .filter_map(|(subnode_id, _)| Subnodes::<T>::get(subnode_id))
                    .filter(|s| s.status == SubnodeStatus::Failed)
                    .count() as u32;

                Self::deposit_event(Event::AutoHealingInitiated {
                    cluster_id,
                    failed_count,
                    active_remaining: cluster.active_subnodes,
                });

                for (subnode_id, _) in ClusterSubnodes::<T>::iter_prefix(cluster_id) {
                    Subnodes::<T>::mutate(subnode_id, |subnode| {
                        if let Some(ref mut s) = subnode {
                            if s.status == SubnodeStatus::Failed {
                                s.status = SubnodeStatus::Inactive;
                                s.consecutive_misses = 0;
                                s.health_score = 50;
                                s.last_heartbeat = block_number;
                            }
                        }
                    });

                    FusedHealth::<T>::mutate(subnode_id, |maybe_health| {
                        if let Some(ref mut health) = maybe_health {
                            let weights = GlobalFusionWeights::<T>::get();
                            health.update_heartbeat(50, block_u64, &weights);
                        }
                    });
                }
            }
        }

        fn check_fusion_healing_triggers(current_block: u64) {
            for (subnode_id, health) in FusedHealth::<T>::iter() {
                if let Some(trigger) = fusion::should_trigger_healing(&health, current_block) {
                    let previous_score = health.fused_score;

                    Self::deposit_event(Event::FusionHealingTriggered {
                        subnode_id,
                        trigger,
                        previous_score,
                    });

                    if health.is_critical() {
                        Subnodes::<T>::mutate(subnode_id, |subnode| {
                            if let Some(ref mut s) = subnode {
                                if s.status == SubnodeStatus::Active {
                                    s.status = SubnodeStatus::Failed;
                                    s.health_score = 0;

                                    Clusters::<T>::mutate(s.cluster, |cluster| {
                                        if let Some(ref mut c) = cluster {
                                            c.active_subnodes = c.active_subnodes.saturating_sub(1);
                                            if c.active_subnodes < T::MinSubnodes::get() {
                                                c.status = ClusterStatus::Degraded;
                                            }
                                        }
                                    });

                                    ActiveSubnodeCount::<T>::mutate(|count| {
                                        *count = count.saturating_sub(1)
                                    });

                                    Self::deposit_event(Event::SubnodeFailed {
                                        subnode_id,
                                        cluster_id: s.cluster,
                                        consecutive_misses: s.consecutive_misses,
                                    });
                                }
                            }
                        });
                    }
                }
            }
        }

        pub fn get_total_active_subnodes() -> u32 {
            ActiveSubnodeCount::<T>::get()
        }

        /// Run diagnostics on a subnode and generate a report.
        pub fn run_diagnostics(subnode_id: SubnodeId) -> Option<DiagnosticReport<BlockNumberFor<T>>> {
            let subnode = Subnodes::<T>::get(subnode_id)?;
            let health = FusedHealth::<T>::get(subnode_id);
            let block_number = frame_system::Pallet::<T>::block_number();

            // Perform health checks
            let checks = DiagnosticChecks {
                heartbeat_ok: subnode.consecutive_misses < T::MaxConsecutiveMisses::get(),
                device_observations_ok: health.as_ref()
                    .map(|h| h.device_metrics.total_observations > 0)
                    .unwrap_or(false),
                position_consistency_ok: health.as_ref()
                    .map(|h| h.position_metrics.position_variance < 5000)
                    .unwrap_or(true),
                cluster_connectivity_ok: subnode.status == SubnodeStatus::Active,
                fused_health_score: health.as_ref().map(|h| h.fused_score).unwrap_or(0),
            };

            // Determine recommended actions
            let mut actions: BoundedVec<DiagnosticAction, MaxDiagnosticActions> = BoundedVec::new();

            if !checks.heartbeat_ok {
                let _ = actions.try_push(DiagnosticAction::RestartHeartbeat);
            }
            if !checks.device_observations_ok {
                let _ = actions.try_push(DiagnosticAction::ClearDeviceCache);
            }
            if !checks.position_consistency_ok {
                let _ = actions.try_push(DiagnosticAction::RecalibratePosition);
            }
            if subnode.status == SubnodeStatus::Failed {
                let _ = actions.try_push(DiagnosticAction::RotateAuthProfile);
                let _ = actions.try_push(DiagnosticAction::ReregisterCluster);
            }
            if checks.fused_health_score < 30 {
                let _ = actions.try_push(DiagnosticAction::ResetFusedHealth);
            }

            // Calculate severity
            let severity = Self::calculate_severity(&checks, &subnode);

            if severity == DiagnosticSeverity::Critical || severity == DiagnosticSeverity::Failed {
                let _ = actions.try_push(DiagnosticAction::EscalateOperator);
            }

            Self::deposit_event(Event::DiagnosticReportGenerated {
                subnode_id,
                severity,
                actions_count: actions.len() as u32,
            });

            Some(DiagnosticReport {
                subnode_id,
                checks,
                actions,
                severity,
                generated_at: block_number,
            })
        }

        /// Calculate diagnostic severity based on checks and subnode state.
        fn calculate_severity(checks: &DiagnosticChecks, subnode: &Subnode<T>) -> DiagnosticSeverity {
            if subnode.status == SubnodeStatus::Failed {
                return DiagnosticSeverity::Failed;
            }

            let issues_count = [
                !checks.heartbeat_ok,
                !checks.device_observations_ok,
                !checks.position_consistency_ok,
                !checks.cluster_connectivity_ok,
            ].iter().filter(|&&x| x).count();

            match issues_count {
                0 if checks.fused_health_score >= 70 => DiagnosticSeverity::Healthy,
                0 | 1 if checks.fused_health_score >= 40 => DiagnosticSeverity::Warning,
                _ if checks.fused_health_score < 30 => DiagnosticSeverity::Critical,
                _ => DiagnosticSeverity::Warning,
            }
        }

        /// Apply auto-fix actions to a subnode.
        pub fn apply_auto_fix(subnode_id: SubnodeId, actions: &[DiagnosticAction]) -> DispatchResult {
            let block_number = frame_system::Pallet::<T>::block_number();
            let block_u64: u64 = block_number.try_into().unwrap_or(0);
            let mut applied_count: u32 = 0;

            for action in actions {
                match action {
                    DiagnosticAction::RestartHeartbeat => {
                        Subnodes::<T>::mutate(subnode_id, |s| {
                            if let Some(subnode) = s {
                                subnode.last_heartbeat = block_number;
                                subnode.consecutive_misses = 0;
                            }
                        });
                        applied_count += 1;
                    },
                    DiagnosticAction::ResetFusedHealth => {
                        FusedHealth::<T>::mutate(subnode_id, |h| {
                            if let Some(health) = h {
                                let weights = GlobalFusionWeights::<T>::get();
                                // Reset heartbeat and recalculate fused score
                                health.update_heartbeat(50, block_u64, &weights);
                            }
                        });
                        applied_count += 1;
                    },
                    DiagnosticAction::ReregisterCluster => {
                        Subnodes::<T>::mutate(subnode_id, |s| {
                            if let Some(subnode) = s {
                                if subnode.status == SubnodeStatus::Failed {
                                    subnode.status = SubnodeStatus::Inactive;
                                    subnode.health_score = 50;
                                }
                            }
                        });
                        applied_count += 1;
                    },
                    DiagnosticAction::RecalibratePosition => {
                        FusedHealth::<T>::mutate(subnode_id, |h| {
                            if let Some(health) = h {
                                health.position_metrics.position_variance = 0;
                            }
                        });
                        applied_count += 1;
                    },
                    DiagnosticAction::RotateAuthProfile => {
                        // Auth profile rotation is external - just record intent
                        applied_count += 1;
                    },
                    DiagnosticAction::ClearDeviceCache => {
                        // Device cache is external - just reset metrics
                        FusedHealth::<T>::mutate(subnode_id, |h| {
                            if let Some(health) = h {
                                health.device_metrics.total_observations = 0;
                            }
                        });
                        applied_count += 1;
                    },
                    DiagnosticAction::EscalateOperator => {
                        if let Some(subnode) = Subnodes::<T>::get(subnode_id) {
                            let severity = if subnode.status == SubnodeStatus::Failed {
                                DiagnosticSeverity::Failed
                            } else {
                                DiagnosticSeverity::Critical
                            };
                            Self::deposit_event(Event::OperatorEscalationRequired {
                                subnode_id,
                                reason: severity,
                            });
                        }
                        applied_count += 1;
                    },
                }
            }

            Self::deposit_event(Event::AutoFixApplied {
                subnode_id,
                actions_applied: applied_count,
            });

            Ok(())
        }

        /// Prune inactive subnodes that have been failed for too long.
        /// Returns the number of subnodes pruned.
        pub fn prune_inactive_subnodes(current_block: BlockNumberFor<T>) -> u32 {
            let inactive_threshold = T::HeartbeatTimeoutBlocks::get() * 10u32.into();
            let mut pruned: u32 = 0;

            for (subnode_id, subnode) in Subnodes::<T>::iter() {
                let inactive_blocks = current_block.saturating_sub(subnode.last_heartbeat);

                if inactive_blocks >= inactive_threshold && subnode.status == SubnodeStatus::Failed {
                    // Remove subnode from all storage
                    Subnodes::<T>::remove(subnode_id);
                    FusedHealth::<T>::remove(subnode_id);
                    ClusterSubnodes::<T>::remove(subnode.cluster, subnode_id);
                    OperatorSubnodes::<T>::remove(subnode.operator, subnode_id);

                    pruned += 1;

                    Self::deposit_event(Event::SubnodePruned { subnode_id });
                }
            }

            pruned
        }
    }
}
