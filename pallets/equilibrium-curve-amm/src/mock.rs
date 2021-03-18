use crate as curve_amm;
use crate::traits::Const;
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    parameter_types,
    traits::{Currency, OnUnbalanced},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::traits::Saturating;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    ModuleId,
};
use sp_runtime::{FixedPointNumber, FixedU128};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Module, Call, Storage, Event<T>},
        CurveAmm: curve_amm::{Module, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

type AccountId = u64;

impl system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
}

parameter_types! {
    pub const CreationFee: Balance = 999;
    pub const CurveAmmModuleId: ModuleId = ModuleId(*b"eq/crvam");
}

pub type Balance = u128;
type Number = FixedU128;
type IntermediateNumber = u128;
pub struct ConstFixedU128;

impl Const<FixedU128> for ConstFixedU128 {
    fn zero() -> FixedU128 {
        FixedU128::zero()
    }

    fn one() -> FixedU128 {
        FixedU128::one()
    }

    fn prec() -> FixedU128 {
        FixedU128::saturating_from_rational(1, 1_000_000)
    }
}

type AssetId = i64;

pub struct EmptyAssets;

impl curve_amm::traits::Assets<AssetId, Balance, AccountId> for EmptyAssets {
    fn create_asset() -> Result<AssetId, DispatchError> {
        Ok(0)
    }

    fn mint(asset: AssetId, dest: AccountId, amount: Balance) -> DispatchResult {
        Ok(())
    }

    fn burn(asset: AssetId, dest: AccountId, amount: Balance) -> DispatchResult {
        Ok(())
    }

    fn transfer(
        asset: AssetId,
        source: AccountId,
        dest: AccountId,
        amount: Balance,
    ) -> DispatchResult {
        Ok(())
    }

    fn balance(asset: AssetId, who: AccountId) -> Balance {
        0
    }

    fn total_issuance(asset: AssetId) -> Balance {
        0
    }
}

pub struct EmptyUnbalanceHandler;

type Imbalance = <pallet_balances::Pallet<Test> as Currency<AccountId>>::NegativeImbalance;

impl OnUnbalanced<Imbalance> for EmptyUnbalanceHandler {}

impl curve_amm::Config for Test {
    type Event = Event;
    type AssetId = i64;
    type Balance = Balance;
    type Currency = Balances;
    type CreationFee = CreationFee;
    type Assets = EmptyAssets;
    type OnUnbalanced = EmptyUnbalanceHandler;
    type ModuleId = CurveAmmModuleId;

    type Number = Number;
    type IntermediateNumber = IntermediateNumber;
    type Const = ConstFixedU128;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into()
}
