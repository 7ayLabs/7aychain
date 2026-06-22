//! Runtime API trait definitions for 7aychain DePIN RPC endpoints.
//!
//! These traits define the interface between the RPC node layer and the
//! runtime. Each trait corresponds to a DePIN subsystem:
//!
//! - `PresenceApi` -- query presence records by actor and epoch
//! - `EpochApi` -- query current epoch state
//! - `ValidatorApi` -- query validator registration and status
//! - `DeviceApi` -- query device health metrics
//!
//! Response types are concrete (not generic over `T: Config`) so they can
//! be SCALE-encoded across the runtime boundary and JSON-serialized by the
//! RPC layer.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

extern crate alloc;

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_runtime::RuntimeDebug;

// =============================================================================
// RPC Response Types
// =============================================================================
// These are concrete types (no generics over Config) designed to cross the
// runtime API boundary via SCALE and be serialized to JSON by the node RPC.

/// Presence record returned by `presence_getState`.
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
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct RpcPresenceRecord {
    /// Actor identity (H256).
    pub actor: H256,
    /// Epoch identifier.
    pub epoch: u64,
    /// Current state as a string tag.
    pub state: RpcPresenceState,
    /// Block number at which presence was declared.
    pub declared_at: Option<u32>,
    /// Block number at which presence was validated.
    pub validated_at: Option<u32>,
    /// Block number at which presence was finalized.
    pub finalized_at: Option<u32>,
    /// Number of validator votes received.
    pub vote_count: u32,
}

/// Presence state enum for RPC responses.
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
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum RpcPresenceState {
    None,
    Declared,
    Validated,
    Finalized,
    Slashed,
}

/// Current epoch info returned by `epoch_current`.
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
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct RpcEpochInfo {
    /// Current epoch identifier.
    pub epoch_id: u64,
    /// Current epoch state.
    pub state: RpcEpochState,
    /// Block at which this epoch started.
    pub start_block: u32,
    /// Block at which this epoch ends.
    pub end_block: u32,
    /// Number of registered participants.
    pub participant_count: u32,
}

/// Epoch state enum for RPC responses.
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
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum RpcEpochState {
    Scheduled,
    Active,
    Closed,
    Finalized,
}

/// Validator status returned by `validator_status`.
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
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct RpcValidatorInfo {
    /// Validator identity (H256).
    pub id: H256,
    /// Staked balance (as u128 for JSON-safe representation).
    pub stake: u128,
    /// Current status.
    pub status: RpcValidatorStatus,
    /// Block at which the validator registered.
    pub registered_at: u32,
    /// Whether the validator is currently unbonding.
    pub is_unbonding: bool,
}

/// Validator status enum for RPC responses.
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
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum RpcValidatorStatus {
    Inactive,
    Active,
    Recovering,
    Suspended,
}

/// Device health metrics returned by `device_health`.
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
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct RpcDeviceHealth {
    /// Device identifier (numeric).
    pub device_id: u64,
    /// Device status string.
    pub status: RpcDeviceStatus,
    /// Trust score (0-100).
    pub trust_score: u8,
    /// Health score from heartbeat monitoring (0-100).
    pub health_score: u8,
    /// Number of consecutive heartbeat misses.
    pub consecutive_misses: u32,
    /// Last heartbeat sequence number.
    pub last_heartbeat_seq: u64,
    /// Whether the device is currently online.
    pub is_online: bool,
}

/// Device status enum for RPC responses.
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
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum RpcDeviceStatus {
    Pending,
    Active,
    Suspended,
    Revoked,
    Compromised,
    Offline,
}

/// Carrier service status returned by `carrier_numberStatus`.
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
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct RpcCarrierStatus {
    /// Number commitment identifier.
    pub number_id: H256,
    /// Subscriber actor identity.
    pub owner: H256,
    /// Current device binding, when service is active.
    pub device_id: Option<u64>,
    /// Serving region identifier.
    pub region_id: H256,
    /// Current service status.
    pub status: RpcCarrierNumberStatus,
    /// Epoch that authorized the current device binding.
    pub activation_epoch: Option<u64>,
    /// Block when service was first activated.
    pub activated_at: Option<u32>,
    /// Service lease end block for the active subscriber binding.
    pub service_lease_until: Option<u32>,
    /// Pending service request epoch, if any.
    pub pending_epoch: Option<u64>,
    /// Average signal quality score (0-100), when aggregated.
    pub avg_signal_score: Option<u8>,
    /// Number of signal quality samples.
    pub signal_sample_count: Option<u32>,
    /// Witness count for the current pending request or latest finalized lease.
    pub witness_count: Option<u32>,
    /// Current provisioning lifecycle state.
    pub provisioning_state: RpcCarrierProvisioningState,
    /// Opaque provisioning receipt written back by the trusted bridge.
    pub provisioning_receipt: Option<H256>,
    /// Subscriber profile commitment bound to the active device.
    pub sim_profile_commitment: Option<H256>,
}

/// Carrier number status enum for RPC responses.
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
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum RpcCarrierNumberStatus {
    Reserved,
    ActivationPending,
    Activated,
    Suspended,
    RecoveryPending,
    Recovered,
    Revoked,
}

/// Carrier provisioning lifecycle state for bridge-aware line activation.
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
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum RpcCarrierProvisioningState {
    None,
    Pending,
    Provisioned,
    Failed,
}

// =============================================================================
// Runtime API Trait Declarations
// =============================================================================

sp_api::decl_runtime_apis! {
    /// Presence subsystem queries for the DePIN RPC layer.
    pub trait PresenceApi {
        /// Returns the presence record for a given actor in a given epoch.
        /// Returns `None` if no presence exists for that (actor, epoch) pair.
        fn get_presence_state(actor_id: H256, epoch_id: u64)
            -> Option<RpcPresenceRecord>;
    }

    /// Epoch subsystem queries for the DePIN RPC layer.
    pub trait EpochApi {
        /// Returns information about the current epoch.
        fn current_epoch() -> Option<RpcEpochInfo>;
    }

    /// Validator subsystem queries for the DePIN RPC layer.
    pub trait ValidatorApi {
        /// Returns validator registration info for the given validator ID.
        /// Returns `None` if the validator is not registered.
        fn validator_status(validator_id: H256) -> Option<RpcValidatorInfo>;
    }

    /// Device subsystem queries for the DePIN RPC layer.
    pub trait DeviceApi {
        /// Returns health metrics for the given device.
        /// The `device_id` is the H256 hash of the device public key.
        /// Returns `None` if the device is not registered.
        fn device_health(device_id: H256) -> Option<RpcDeviceHealth>;
    }

    /// Carrier subsystem queries for native-number service state.
    pub trait CarrierApi {
        /// Returns carrier status for the given number commitment.
        fn carrier_number_status(number_id: H256) -> Option<RpcCarrierStatus>;
    }
}
