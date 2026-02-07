#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "256"]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H256};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, One, Verify},
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, MultiSignature,
};
extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;
use sp_version::RuntimeVersion;

use frame_support::{
    construct_runtime,
    dispatch::DispatchClass,
    genesis_builder_helper::{build_state, get_preset},
    parameter_types,
    traits::{ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, KeyOwnerProofSystem},
    weights::{
        constants::{
            BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
        },
        IdentityFee, Weight,
    },
};
use frame_system::limits::{BlockLength, BlockWeights};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
pub use sp_runtime::{Perbill, Permill};

pub type BlockNumber = u32;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Balance = u128;
pub type Nonce = u32;
pub type Hash = H256;

pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    pub type BlockId = generic::BlockId<Block>;

    impl_opaque_keys! {
        pub struct SessionKeys {
            pub aura: Aura,
            pub grandpa: Grandpa,
        }
    }
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("seveny"),
    impl_name: create_runtime_str!("seveny-node"),
    authoring_version: 1,
    spec_version: 100,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    system_version: 1,
};

#[cfg(feature = "std")]
pub fn native_version() -> sp_version::NativeVersion {
    sp_version::NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    pub const Version: RuntimeVersion = VERSION;
    pub BlockWeightsValue: BlockWeights = BlockWeights::with_sensible_defaults(
        Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
        NORMAL_DISPATCH_RATIO,
    );
    pub BlockLengthValue: BlockLength = BlockLength::max_with_normal_ratio(
        5 * 1024 * 1024,
        NORMAL_DISPATCH_RATIO,
    );
    pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Runtime {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = BlockWeightsValue;
    type BlockLength = BlockLengthValue;
    type DbWeight = RocksDbWeight;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = Nonce;
    type Hash = Hash;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = sp_runtime::traits::AccountIdLookup<AccountId, ()>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = Version;
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type RuntimeTask = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
    type ExtensionsWeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 3000;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxAuthorities: u32 = 100;
    pub const MaxNominators: u32 = 256;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = MaxAuthorities;
    type AllowMultipleBlocksPerSlot = ConstBool<false>;
    type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Runtime>;
}

impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
    type MaxNominators = MaxNominators;
    type MaxSetIdSessionEntries = ConstU64<0>;
    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 500;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = ConstU32<1>;
    type DoneSlashHandler = ();
}

parameter_types! {
    pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = FungibleAdapter<Balances, ()>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightToFee = IdentityFee<Balance>;
    type LengthToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
    type WeightInfo = ();
}

impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxVotesPerPresence: u32 = 100;
    pub const DefaultQuorumThreshold: u32 = 2;
    pub const DefaultQuorumTotal: u32 = 3;
    pub const CommitRevealDelay: BlockNumber = 10;
    pub const RevealWindow: BlockNumber = 20;
}

impl pallet_presence::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxVotesPerPresence = MaxVotesPerPresence;
    type DefaultQuorumThreshold = DefaultQuorumThreshold;
    type DefaultQuorumTotal = DefaultQuorumTotal;
    type CommitRevealDelay = CommitRevealDelay;
    type RevealWindow = RevealWindow;
}

parameter_types! {
    pub const EpochDuration: BlockNumber = 100;
    pub const MinEpochDuration: BlockNumber = 10;
    pub const MaxEpochDuration: BlockNumber = 1000;
    pub const GracePeriod: BlockNumber = 10;
}

impl pallet_epoch::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type EpochDuration = EpochDuration;
    type MinEpochDuration = MinEpochDuration;
    type MaxEpochDuration = MaxEpochDuration;
    type GracePeriod = GracePeriod;
}

parameter_types! {
    pub const MinStake: Balance = 10_000;
    pub const MaxValidators: u32 = 100;
    pub const BondingDuration: BlockNumber = 100;
    pub const SlashDeferDuration: BlockNumber = 50;
}

impl pallet_validator::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type Currency = Balances;
    type MinStake = MinStake;
    type MaxValidators = MaxValidators;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
}

parameter_types! {
    pub const MaxEvidencePerDispute: u32 = 10;
    pub const DisputeResolutionPeriod: BlockNumber = 50;
    pub const MinEvidenceRequired: u32 = 1;
    pub const MaxDisputesPerValidator: u32 = 5;
    pub const MaxOpenDisputes: u32 = 100;
}

impl pallet_dispute::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxEvidencePerDispute = MaxEvidencePerDispute;
    type DisputeResolutionPeriod = DisputeResolutionPeriod;
    type MinEvidenceRequired = MinEvidenceRequired;
    type MaxDisputesPerValidator = MaxDisputesPerValidator;
    type MaxOpenDisputes = MaxOpenDisputes;
}

parameter_types! {
    pub const MaxCapabilitiesPerActor: u32 = 100;
    pub const MaxDelegationDepth: u32 = 5;
    pub const DefaultCapabilityDuration: BlockNumber = 1000;
    pub const MaxCapabilitiesPerResource: u32 = 50;
}

impl pallet_governance::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxCapabilitiesPerActor = MaxCapabilitiesPerActor;
    type MaxDelegationDepth = MaxDelegationDepth;
    type DefaultCapabilityDuration = DefaultCapabilityDuration;
    type MaxCapabilitiesPerResource = MaxCapabilitiesPerResource;
}

parameter_types! {
    pub const MaxRelationshipsPerActor: u32 = 50;
    pub const MaxDiscoveryResults: u32 = 100;
    pub const DiscoveryRateLimitBlocks: BlockNumber = 10;
    pub const RelationshipExpiryBlocks: BlockNumber = 10000;
    pub const MaxTrustLevel: u8 = 100;
}

impl pallet_semantic::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxRelationshipsPerActor = MaxRelationshipsPerActor;
    type MaxDiscoveryResults = MaxDiscoveryResults;
    type DiscoveryRateLimitBlocks = DiscoveryRateLimitBlocks;
    type RelationshipExpiryBlocks = RelationshipExpiryBlocks;
    type MaxTrustLevel = MaxTrustLevel;
}

parameter_types! {
    pub const BoomerangTimeoutBlocks: BlockNumber = 10;
    pub const MaxExtensionBlocks: BlockNumber = 100;
    pub const MaxHopsPerPath: u32 = 10;
    pub const MaxActivePaths: u32 = 100;
}

impl pallet_boomerang::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type BoomerangTimeoutBlocks = BoomerangTimeoutBlocks;
    type MaxExtensionBlocks = MaxExtensionBlocks;
    type MaxHopsPerPath = MaxHopsPerPath;
    type MaxActivePaths = MaxActivePaths;
}

parameter_types! {
    pub const PatternThreshold: u32 = 3;
    pub const MaxBehaviorsPerActor: u32 = 50;
    pub const MaxPatterns: u32 = 100;
    pub const BehaviorExpiryBlocks: BlockNumber = 10000;
    pub const ScoreIncreasePerMatch: u8 = 5;
}

impl pallet_autonomous::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type PatternThreshold = PatternThreshold;
    type MaxBehaviorsPerActor = MaxBehaviorsPerActor;
    type MaxPatterns = MaxPatterns;
    type BehaviorExpiryBlocks = BehaviorExpiryBlocks;
    type ScoreIncreasePerMatch = ScoreIncreasePerMatch;
}

parameter_types! {
    pub const ActivationThreshold: Perbill = Perbill::from_percent(45);
    pub const DeactivationThreshold: Perbill = Perbill::from_percent(20);
    pub const DeactivationDurationBlocks: BlockNumber = 100;
    pub const MaxSubnodesPerCluster: u32 = 8;
    pub const MinSubnodes: u32 = 2;
    pub const ScalingCooldownBlocks: BlockNumber = 50;
}

impl pallet_octopus::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type ActivationThreshold = ActivationThreshold;
    type DeactivationThreshold = DeactivationThreshold;
    type DeactivationDurationBlocks = DeactivationDurationBlocks;
    type MaxSubnodesPerCluster = MaxSubnodesPerCluster;
    type MinSubnodes = MinSubnodes;
    type ScalingCooldownBlocks = ScalingCooldownBlocks;
}

parameter_types! {
    pub const MaxDevicesPerActor: u32 = 10;
    pub const AttestationValidityBlocks: BlockNumber = 1000;
    pub const InitialTrustScore: u8 = 50;
}

impl pallet_device::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxDevicesPerActor = MaxDevicesPerActor;
    type AttestationValidityBlocks = AttestationValidityBlocks;
    type InitialTrustScore = InitialTrustScore;
}

parameter_types! {
    pub const MinThreshold: u32 = 2;
    pub const MinRingSize: u32 = 3;
    pub const MaxRingSize: u32 = 10;
    pub const RecoveryPeriodBlocks: BlockNumber = 100;
    pub const MaxVaultsPerActor: u32 = 5;
}

impl pallet_vault::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MinThreshold = MinThreshold;
    type MinRingSize = MinRingSize;
    type MaxRingSize = MaxRingSize;
    type RecoveryPeriodBlocks = RecoveryPeriodBlocks;
    type MaxVaultsPerActor = MaxVaultsPerActor;
}

parameter_types! {
    pub const MaxProofSize: u32 = 1024;
    pub const MaxVerificationsPerBlock: u32 = 100;
}

impl pallet_zk::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxProofSize = MaxProofSize;
    type MaxVerificationsPerBlock = MaxVerificationsPerBlock;
}

parameter_types! {
    pub const MaxDataSize: u32 = 10_000;
    pub const MaxEntriesPerActor: u32 = 100;
    pub const MaxEntriesPerEpoch: u32 = 10_000;
    pub const DefaultRetentionBlocks: BlockNumber = 1000;
}

impl pallet_storage::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxDataSize = MaxDataSize;
    type MaxEntriesPerActor = MaxEntriesPerActor;
    type MaxEntriesPerEpoch = MaxEntriesPerEpoch;
    type DefaultRetentionBlocks = DefaultRetentionBlocks;
}

parameter_types! {
    pub const KeyDestructionTimeoutBlocks: BlockNumber = 100;
    pub const MinDestructionAttestations: u32 = 3;
    pub const RotationCooldownBlocks: BlockNumber = 50;
}

impl pallet_lifecycle::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type KeyDestructionTimeoutBlocks = KeyDestructionTimeoutBlocks;
    type MinDestructionAttestations = MinDestructionAttestations;
    type RotationCooldownBlocks = RotationCooldownBlocks;
}

construct_runtime!(
    pub enum Runtime {
        System: frame_system,
        Timestamp: pallet_timestamp,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,
        Sudo: pallet_sudo,

        Presence: pallet_presence,
        Epoch: pallet_epoch,
        Validator: pallet_validator,
        Dispute: pallet_dispute,
        Governance: pallet_governance,
        Semantic: pallet_semantic,
        Boomerang: pallet_boomerang,
        Autonomous: pallet_autonomous,
        Octopus: pallet_octopus,
        Device: pallet_device,
        Vault: pallet_vault,
        Zk: pallet_zk,
        Storage: pallet_storage,
        Lifecycle: pallet_lifecycle,
    }
);

pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
pub type UncheckedExtrinsic =
    generic::UncheckedExtrinsic<sp_runtime::MultiAddress<AccountId, ()>, RuntimeCall, Signature, SignedExtra>;
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    frame_support::define_benchmarks!(
        [frame_system, SystemBench::<Runtime>]
        [pallet_balances, Balances]
        [pallet_timestamp, Timestamp]
    );
}

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: <Block as BlockT>::LazyBlock) {
            Executive::execute_block(block);
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
            Runtime::metadata_at_version(version)
        }

        fn metadata_versions() -> alloc::vec::Vec<u32> {
            Runtime::metadata_versions()
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: <Block as BlockT>::LazyBlock,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            pallet_aura::Authorities::<Runtime>::get().into_inner()
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            opaque::SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> sp_consensus_grandpa::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            _equivocation_proof: sp_consensus_grandpa::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            _key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            None
        }

        fn generate_key_ownership_proof(
            _set_id: sp_consensus_grandpa::SetId,
            _authority_id: GrandpaId,
        ) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
            None
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
            TransactionPayment::weight_to_fee(weight)
        }
        fn query_length_to_fee(length: u32) -> Balance {
            TransactionPayment::length_to_fee(length)
        }
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            get_preset::<RuntimeGenesisConfig>(id, |_| None)
        }

        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            vec![]
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
            let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, BlockWeightsValue::get().max_block)
        }

        fn execute_block(
            block: Block,
            state_root_check: bool,
            signature_check: bool,
            select: frame_try_runtime::TryStateSelect,
        ) -> Weight {
            Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
        }
    }
}
