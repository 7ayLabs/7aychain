//! DePIN RPC endpoint definitions for 7aychain.
//!
//! Provides four custom RPC namespaces:
//!
//! - `presence_getState(actor_id, epoch_id)` -- query presence records
//! - `epoch_current()` -- query current epoch info
//! - `validator_status(validator_id)` -- query validator registration
//! - `device_health(device_id)` -- query device health metrics
//!
//! Each endpoint delegates to a runtime API call, executing the query
//! against the best (latest finalized) block state.

use std::sync::Arc;

use jsonrpsee::{core::RpcResult, proc_macros::rpc, types::ErrorObjectOwned, RpcModule};
use sc_transaction_pool_api::TransactionPool;
use seveny_runtime::{opaque::Block, AccountId, Balance, Nonce};
use seveny_runtime_api::{
    CarrierApi, DeviceApi, EpochApi, PresenceApi, RpcCarrierStatus, RpcDeviceHealth, RpcEpochInfo,
    RpcPresenceRecord, RpcValidatorInfo, ValidatorApi,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_core::H256;

use crate::carrier_signal::{CarrierSignalHandle, CarrierSignalSample};

// =============================================================================
// Error helpers
// =============================================================================

/// Internal RPC error code for runtime API call failures.
const RUNTIME_ERROR: i32 = 1;

fn runtime_err(msg: impl ToString) -> ErrorObjectOwned {
    ErrorObjectOwned::owned(RUNTIME_ERROR, "Runtime error", Some(msg.to_string()))
}

// =============================================================================
// Presence RPC
// =============================================================================

#[rpc(client, server)]
pub trait PresenceRpc {
    /// Returns the presence record for a given actor in a given epoch.
    ///
    /// Parameters:
    /// - `actor_id`: H256 hex string identifying the actor
    /// - `epoch_id`: Epoch number (u64)
    ///
    /// Returns `null` if no presence exists for that (actor, epoch) pair.
    #[method(name = "presence_getState")]
    fn get_state(&self, actor_id: H256, epoch_id: u64) -> RpcResult<Option<RpcPresenceRecord>>;
}

pub struct PresenceRpcImpl<C> {
    client: Arc<C>,
}

impl<C> PresenceRpcImpl<C> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }
}

impl<C> PresenceRpcServer for PresenceRpcImpl<C>
where
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
    C: Send + Sync,
    C::Api: seveny_runtime_api::PresenceApi<Block>,
{
    fn get_state(&self, actor_id: H256, epoch_id: u64) -> RpcResult<Option<RpcPresenceRecord>> {
        let best = self.client.info().best_hash;
        self.client
            .runtime_api()
            .get_presence_state(best, actor_id, epoch_id)
            .map_err(|e| runtime_err(e))
    }
}

// =============================================================================
// Epoch RPC
// =============================================================================

#[rpc(client, server)]
pub trait EpochRpc {
    /// Returns information about the current epoch.
    ///
    /// Returns `null` if no epoch has been initialized yet.
    #[method(name = "epoch_current")]
    fn current(&self) -> RpcResult<Option<RpcEpochInfo>>;
}

pub struct EpochRpcImpl<C> {
    client: Arc<C>,
}

impl<C> EpochRpcImpl<C> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }
}

impl<C> EpochRpcServer for EpochRpcImpl<C>
where
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
    C: Send + Sync,
    C::Api: seveny_runtime_api::EpochApi<Block>,
{
    fn current(&self) -> RpcResult<Option<RpcEpochInfo>> {
        let best = self.client.info().best_hash;
        self.client
            .runtime_api()
            .current_epoch(best)
            .map_err(|e| runtime_err(e))
    }
}

// =============================================================================
// Validator RPC
// =============================================================================

#[rpc(client, server)]
pub trait ValidatorRpc {
    /// Returns validator registration info for the given validator ID.
    ///
    /// Parameters:
    /// - `validator_id`: H256 hex string identifying the validator
    ///
    /// Returns `null` if the validator is not registered.
    #[method(name = "validator_status")]
    fn status(&self, validator_id: H256) -> RpcResult<Option<RpcValidatorInfo>>;
}

pub struct ValidatorRpcImpl<C> {
    client: Arc<C>,
}

impl<C> ValidatorRpcImpl<C> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }
}

impl<C> ValidatorRpcServer for ValidatorRpcImpl<C>
where
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
    C: Send + Sync,
    C::Api: seveny_runtime_api::ValidatorApi<Block>,
{
    fn status(&self, validator_id: H256) -> RpcResult<Option<RpcValidatorInfo>> {
        let best = self.client.info().best_hash;
        self.client
            .runtime_api()
            .validator_status(best, validator_id)
            .map_err(|e| runtime_err(e))
    }
}

// =============================================================================
// Device RPC
// =============================================================================

#[rpc(client, server)]
pub trait DeviceRpc {
    /// Returns health metrics for a registered device.
    ///
    /// Parameters:
    /// - `device_id`: H256 hex string of the device public key hash
    ///
    /// Returns `null` if the device is not registered.
    #[method(name = "device_health")]
    fn health(&self, device_id: H256) -> RpcResult<Option<RpcDeviceHealth>>;
}

// =============================================================================
// Carrier RPC
// =============================================================================

#[rpc(client, server)]
pub trait CarrierRpc {
    /// Returns carrier state for a number commitment.
    ///
    /// Parameters:
    /// - `number_id`: H256 hex string identifying the number commitment
    ///
    /// Returns `null` if the number is not registered.
    #[method(name = "carrier_numberStatus")]
    fn number_status(&self, number_id: H256) -> RpcResult<Option<RpcCarrierStatus>>;
}

pub struct CarrierRpcImpl<C> {
    client: Arc<C>,
}

impl<C> CarrierRpcImpl<C> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }
}

impl<C> CarrierRpcServer for CarrierRpcImpl<C>
where
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
    C: Send + Sync,
    C::Api: seveny_runtime_api::CarrierApi<Block>,
{
    fn number_status(&self, number_id: H256) -> RpcResult<Option<RpcCarrierStatus>> {
        let best = self.client.info().best_hash;
        self.client
            .runtime_api()
            .carrier_number_status(best, number_id)
            .map_err(|e| runtime_err(e))
    }
}

pub struct DeviceRpcImpl<C> {
    client: Arc<C>,
}

impl<C> DeviceRpcImpl<C> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }
}

impl<C> DeviceRpcServer for DeviceRpcImpl<C>
where
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
    C: Send + Sync,
    C::Api: seveny_runtime_api::DeviceApi<Block>,
{
    fn health(&self, device_id: H256) -> RpcResult<Option<RpcDeviceHealth>> {
        let best = self.client.info().best_hash;
        self.client
            .runtime_api()
            .device_health(best, device_id)
            .map_err(|e| runtime_err(e))
    }
}

// =============================================================================
// Carrier Signal RPC (node-local, not a runtime API)
// =============================================================================

#[rpc(client, server)]
pub trait CarrierSignalRpc {
    /// Returns the most recent carrier-signal sample observed by this node.
    /// Returns `null` if no sample is available (mode disabled or source
    /// has not yet produced data).
    #[method(name = "seveny_currentCarrierSignal")]
    fn current(&self) -> RpcResult<Option<CarrierSignalSample>>;
}

pub struct CarrierSignalRpcImpl {
    handle: CarrierSignalHandle,
}

impl CarrierSignalRpcImpl {
    pub fn new(handle: CarrierSignalHandle) -> Self {
        Self { handle }
    }
}

impl CarrierSignalRpcServer for CarrierSignalRpcImpl {
    fn current(&self) -> RpcResult<Option<CarrierSignalSample>> {
        Ok(self.handle.read())
    }
}

// =============================================================================
// Full RPC builder
// =============================================================================

pub struct FullDeps<C, P> {
    pub client: Arc<C>,
    pub pool: Arc<P>,
    pub carrier_signal: CarrierSignalHandle,
}

pub fn create_full<C, P>(
    deps: FullDeps<C, P>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
    C: Send + Sync + 'static,
    C::Api: sp_block_builder::BlockBuilder<Block>,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: seveny_runtime_api::PresenceApi<Block>,
    C::Api: seveny_runtime_api::EpochApi<Block>,
    C::Api: seveny_runtime_api::ValidatorApi<Block>,
    C::Api: seveny_runtime_api::DeviceApi<Block>,
    C::Api: seveny_runtime_api::CarrierApi<Block>,
    P: TransactionPool + 'static,
{
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
    use substrate_frame_rpc_system::{System, SystemApiServer};

    let mut module = RpcModule::new(());
    let FullDeps {
        client,
        pool,
        carrier_signal,
    } = deps;

    // Standard Substrate RPCs
    module.merge(System::new(client.clone(), pool).into_rpc())?;
    module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

    // DePIN custom RPCs
    module.merge(PresenceRpcImpl::new(client.clone()).into_rpc())?;
    module.merge(EpochRpcImpl::new(client.clone()).into_rpc())?;
    module.merge(ValidatorRpcImpl::new(client.clone()).into_rpc())?;
    module.merge(DeviceRpcImpl::new(client.clone()).into_rpc())?;
    module.merge(CarrierRpcImpl::new(client).into_rpc())?;

    // Carrier signal (node-local)
    module.merge(CarrierSignalRpcImpl::new(carrier_signal).into_rpc())?;

    Ok(module)
}
