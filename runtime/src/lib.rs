#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod curve_amm_weights;

use pallet_grandpa::fg_primitives;
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::traits::Convert;
use sp_runtime::traits::{
    AccountIdLookup, BlakeTwo256, Block as BlockT, IdentifyAccount, NumberFor, Verify,
};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    transaction_validity::{TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, ModuleId, MultiSignature,
};
use sp_runtime::{FixedPointNumber, FixedU128};
use sp_std::convert::{TryFrom, TryInto};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use frame_system::RawOrigin;
use pallet_assets::Call as AssetsCall;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::traits::Dispatchable;
use sp_runtime::FixedI64;
use sp_runtime::MultiAddress;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
    construct_runtime,
    dispatch::{DispatchError, DispatchResult},
    parameter_types,
    traits::{Currency, EnsureOrigin, KeyOwnerProofSystem, OnUnbalanced, Randomness},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        IdentityFee, Weight,
    },
    StorageValue,
};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_transaction_payment::CurrencyAdapter;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

/// Import the equilibrium_curve_amm pallet.
pub use equilibrium_curve_amm;
use equilibrium_curve_amm::traits::CurveAmm as CurveAmmTrait;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u64;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;

    impl_opaque_keys! {
        pub struct SessionKeys {
            pub aura: Aura,
            pub grandpa: Grandpa,
        }
    }
}

pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("node-template"),
    impl_name: create_runtime_str!("node-template"),
    authoring_version: 1,
    spec_version: 1,
    impl_version: 1,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
};

/// This determines the average expected block time that we are targetting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;
    pub const BlockHashCount: BlockNumber = 2400;
    /// We allow for 2 seconds of compute with a 6 second average block time.
    pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
        ::with_sensible_defaults(2 * WEIGHT_PER_SECOND, NORMAL_DISPATCH_RATIO);
    pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
        ::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = ();
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = BlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = BlockLength;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = AccountIdLookup<AccountId, ()>;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// The ubiquitous event type.
    type Event = Event;
    /// The ubiquitous origin type.
    type Origin = Origin;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Version of the runtime.
    type Version = Version;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type PalletInfo = PalletInfo;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = ();
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
}

impl pallet_grandpa::Config for Runtime {
    type Event = Event;
    type Call = Call;

    type KeyOwnerProofSystem = ();

    type KeyOwnerProof =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        GrandpaId,
    )>>::IdentificationTuple;

    type HandleEquivocation = ();

    type WeightInfo = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 500;
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TransactionByteFee: Balance = 1;
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Config for Runtime {
    type Event = Event;
    type Call = Call;
}

type AssetId = u32;

parameter_types! {
    pub const AssetDepositBase: Balance = 1;
    pub const AssetDepositPerZombie: Balance = 1;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = 1;
    pub const MetadataDepositPerByte: Balance = 1;
}

pub struct EnsureCurveAmm;
impl EnsureOrigin<Origin> for EnsureCurveAmm {
    type Success = AccountId;
    fn try_origin(o: Origin) -> Result<Self::Success, Origin> {
        let module_id = CurveAmmModuleId::get();
        let account_id: AccountId = module_id.into_account();

        let result: Result<RawOrigin<AccountId>, Origin> = o.into();

        result.and_then(|o| match o {
            RawOrigin::Signed(id) if id == account_id => Ok(id),
            r => Err(Origin::from(r)),
        })
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn successful_origin() -> Origin {
        let module_id = CurveAmmModuleId::get();
        let account_id: AccountId = module_id.into_account();
        Origin::from(RawOrigin::Signed(account_id))
    }
}

impl pallet_assets::Config for Runtime {
    type Event = Event;
    type Balance = Balance;
    type AssetId = AssetId;
    type Currency = Balances;
    type ForceOrigin = EnsureCurveAmm;
    type AssetDepositBase = AssetDepositBase;
    type AssetDepositPerZombie = AssetDepositPerZombie;
    type StringLimit = StringLimit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const CreationFee: Balance = 999;
    pub const CurveAmmModuleId: ModuleId = ModuleId(*b"eq/crvam");
    pub Precision: FixedU128 = FixedU128::saturating_from_rational(1, 1_000_000);
}

type Number = FixedU128;

pub struct EmptyUnbalanceHandler;

impl OnUnbalanced<<pallet_balances::Pallet<Runtime> as Currency<AccountId>>::NegativeImbalance>
    for EmptyUnbalanceHandler
{
}

pub struct FixedU128Convert;

impl Convert<Permill, FixedU128> for FixedU128Convert {
    fn convert(a: Permill) -> FixedU128 {
        a.into()
    }
}

impl Convert<Balance, FixedU128> for FixedU128Convert {
    fn convert(a: Balance) -> FixedU128 {
        let accuracy = FixedU128::accuracy() / FixedI64::accuracy() as u128;
        FixedU128::from_inner(a as u128 * accuracy)
    }
}

impl Convert<u8, FixedU128> for FixedU128Convert {
    fn convert(a: u8) -> FixedU128 {
        FixedU128::saturating_from_integer(a)
    }
}

impl equilibrium_curve_amm::traits::CheckedConvert<usize, FixedU128> for FixedU128Convert {
    fn convert(a: usize) -> Option<FixedU128> {
        Some(FixedU128::saturating_from_integer(u128::try_from(a).ok()?))
    }
}

impl Convert<FixedU128, Balance> for FixedU128Convert {
    fn convert(a: FixedU128) -> Balance {
        let accuracy = FixedU128::accuracy() / FixedI64::accuracy() as u128;
        // NOTE: Please do not use `unwrap` function in production.
        (a.into_inner() / accuracy).try_into().unwrap()
    }
}

pub struct FrameAssets;

/// NOTE: Please do not use this implementation in production.
/// It has some major issues. But it is a great example on the other hand.
/// Trait `equilibrium_curve_amm::traits::Assets` expects that implementation
/// will generate asset id for the new asset on it's own. But `pallet-assets` in contrast
/// expects that asset id will be provided by the caller. The only thing we can do here
/// is to guess asset id and hope that it is not in use.
impl equilibrium_curve_amm::traits::Assets<AssetId, Balance, AccountId> for FrameAssets {
    fn create_asset(_pool_id: equilibrium_curve_amm::PoolId) -> Result<AssetId, DispatchError> {
        fn random_u32_seed() -> u32 {
            let seed = RandomnessCollectiveFlip::random_seed();
            let seed_bytes = seed.as_fixed_bytes();
            let small_seed_bytes = [seed_bytes[0], seed_bytes[1], seed_bytes[2], seed_bytes[3]];
            let small_seed: u32 = u32::from_le_bytes(small_seed_bytes);

            small_seed
        }

        /// See https://en.wikipedia.org/wiki/Linear_congruential_generator
        fn lcg(seed: u32) -> u32 {
            const A: u32 = 1664525;
            const C: u32 = 1013904223;

            A.overflowing_mul(seed).0.overflowing_add(C).0
        }

        let module_id = CurveAmmModuleId::get();
        let account_id: AccountId = module_id.into_account();
        let raw_origin = RawOrigin::Signed(account_id.clone());
        let origin: Origin = raw_origin.into();

        let multi_address: MultiAddress<AccountId, ()> = MultiAddress::Id(account_id);

        // Guessing unused asset id
        let mut seed = random_u32_seed();
        for _ in 0..100 {
            seed = lcg(seed);

            let call = Call::Assets(AssetsCall::force_create(seed, multi_address.clone(), 0, 1));
            if call.dispatch(origin.clone()).map_err(|x| x.error).is_ok() {
                return Ok(seed);
            }
        }

        Err(DispatchError::Other(&"Out of luck"))
    }

    fn mint(asset: AssetId, dest: &AccountId, amount: Balance) -> DispatchResult {
        let module_id = CurveAmmModuleId::get();
        let account_id: AccountId = module_id.into_account();
        let raw_origin = RawOrigin::Signed(account_id.clone());
        let origin: Origin = raw_origin.into();

        let multi_address: MultiAddress<AccountId, ()> = MultiAddress::Id(dest.clone());

        let call = Call::Assets(AssetsCall::mint(asset, multi_address, amount));
        call.dispatch(origin.clone()).map_err(|x| x.error)?;

        Ok(())
    }

    fn burn(asset: AssetId, dest: &AccountId, amount: Balance) -> DispatchResult {
        let module_id = CurveAmmModuleId::get();
        let account_id: AccountId = module_id.into_account();
        let raw_origin = RawOrigin::Signed(account_id.clone());
        let origin: Origin = raw_origin.into();

        let multi_address: MultiAddress<AccountId, ()> = MultiAddress::Id(dest.clone());

        let call = Call::Assets(AssetsCall::burn(asset, multi_address, amount));
        call.dispatch(origin.clone()).map_err(|x| x.error)?;

        Ok(())
    }

    fn transfer(
        asset: AssetId,
        source: &AccountId,
        dest: &AccountId,
        amount: Balance,
    ) -> DispatchResult {
        let raw_origin = RawOrigin::Signed(source.clone());
        let origin: Origin = raw_origin.into();

        let multi_address: MultiAddress<AccountId, ()> = MultiAddress::Id(dest.clone());

        let call = Call::Assets(AssetsCall::transfer(asset, multi_address, amount));
        call.dispatch(origin.clone()).map_err(|x| x.error)?;

        Ok(())
    }

    fn balance(asset: AssetId, who: &AccountId) -> Balance {
        Assets::balance(asset, who.clone())
    }

    fn total_issuance(asset: AssetId) -> Balance {
        Assets::total_supply(asset)
    }

    fn withdraw_admin_fees(
        pool_id: equilibrium_curve_amm::PoolId,
        amounts: impl Iterator<Item = Balance>,
    ) -> DispatchResult {
        let pool = CurveAmm::pool(pool_id).ok_or(DispatchError::Other(&"Pool not found"))?;
        let assets = pool.assets;

        for (asset, fee) in assets.into_iter().zip(amounts) {
            Self::burn(asset, &CurveAmmModuleId::get().into_account(), fee)?;
        }
        Ok(())
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn create_benchmark_asset() -> AssetId {
        Self::create_asset(Default::default()).unwrap()
    }
}

/// Configure the pallet equilibrium_curve_amm in pallets/equilibrium_curve_amm.
impl equilibrium_curve_amm::Config for Runtime {
    type Event = Event;
    type AssetId = AssetId;
    type Balance = Balance;
    type Currency = pallet_balances::Pallet<Runtime>;
    type CreationFee = CreationFee;
    type Assets = FrameAssets;
    type OnUnbalanced = EmptyUnbalanceHandler;
    type ModuleId = CurveAmmModuleId;

    type Number = Number;
    type Precision = Precision;
    type Convert = FixedU128Convert;
    type AssetChecker = ();
    type WeightInfo = crate::curve_amm_weights::WeightInfo<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkingInit = ();
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
        Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
        Aura: pallet_aura::{Module, Config<T>},
        Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},
        Assets: pallet_assets::{Module, Call, Storage, Event<T>},
        Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
        TransactionPayment: pallet_transaction_payment::{Module, Storage},
        Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},
        // Include the custom logic from the equilibrium_curve_amm pallet in the runtime.
        CurveAmm: equilibrium_curve_amm::{Module, Call, Storage, Event<T>},
    }
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllModules,
>;

impl_runtime_apis! {
    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            Runtime::metadata().into()
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
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }

        fn random_seed() -> <Block as BlockT>::Hash {
            RandomnessCollectiveFlip::random_seed()
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> u64 {
            Aura::slot_duration()
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities()
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

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            _equivocation_proof: fg_primitives::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            _key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            None
        }

        fn generate_key_ownership_proof(
            _set_id: fg_primitives::SetId,
            _authority_id: GrandpaId,
        ) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
            // NOTE: this is the only implementation possible since we've
            // defined our key owner proof type as a bottom type (i.e. a type
            // with no values).
            None
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
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
    }

    impl equilibrium_curve_amm_rpc_runtime_api::EquilibriumCurveAmmApi<Block, Balance> for Runtime {
        fn get_dy(
            pool_id: equilibrium_curve_amm::PoolId,
            i: equilibrium_curve_amm::PoolTokenIndex,
            j: equilibrium_curve_amm::PoolTokenIndex,
            dx: Balance
        ) -> Option<Balance> {
            CurveAmm::get_dy(pool_id, i, j, dx).ok()
        }
        fn get_withdraw_one_coin(
            pool_id: equilibrium_curve_amm::PoolId,
            burn_amount: Balance,
            i: equilibrium_curve_amm::PoolTokenIndex
        ) -> Option<Balance> {
            CurveAmm::get_withdraw_one_coin(pool_id, burn_amount, i).ok()
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

            use frame_system_benchmarking::Module as SystemBench;
            use equilibrium_curve_amm::benchmarking::Module as CurveAmmBench;

            impl frame_system_benchmarking::Config for Runtime {}
            impl equilibrium_curve_amm::benchmarking::Config for Runtime {}

            let whitelist: Vec<TrackedStorageKey> = vec![
                // Block Number
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
                // Total Issuance
                hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
                // Execution Phase
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
                // Event Count
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
                // System Events
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
            ];

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);

            add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
            add_benchmark!(params, batches, pallet_balances, Balances);
            add_benchmark!(params, batches, pallet_timestamp, Timestamp);
            add_benchmark!(params, batches, equilibrium_curve_amm, CurveAmmBench::<Runtime>);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }
}
