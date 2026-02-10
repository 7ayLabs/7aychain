#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;
pub mod weights;

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
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: BlockNumberFor<T>) -> Weight {
            Self::process_deactivations(n);
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

        pub fn get_total_active_subnodes() -> u32 {
            ActiveSubnodeCount::<T>::get()
        }
    }
}
