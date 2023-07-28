use crate as curve_amm;
use crate::traits::{CheckedConvert, CurveAmm as CurveAmmTrait};
use frame_support::PalletId;
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    parameter_types,
    traits::{Currency, OnUnbalanced},
};
use frame_system as system;
use sp_core::H256;
use sp_runtime::traits::Convert;
use sp_runtime::Permill;
use sp_runtime::{
    testing::Header,
    traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
};
use sp_runtime::{FixedI64, FixedPointNumber, FixedU128};
use sp_std::convert::{TryFrom, TryInto};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
        CurveAmm: curve_amm::{Pallet, Call, Storage, Event<T>},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

pub type AccountId = u64;

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
    type HoldIdentifier = ();
    type FreezeIdentifier = ();
    type MaxHolds = ();
    type MaxFreezes = ();
}

parameter_types! {
    pub const CreationFee: Balance = 999;
    pub const CurveAmmModuleId: PalletId = PalletId(*b"eq/crvam");
    pub Precision: FixedU128 = FixedU128::saturating_from_rational(1, 1_000_000_000);
}

pub type Balance = u64;
type Number = FixedU128;

pub type AssetId = i64;

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

impl CheckedConvert<usize, FixedU128> for FixedU128Convert {
    fn convert(a: usize) -> Option<FixedU128> {
        Some(FixedU128::saturating_from_integer(u128::try_from(a).ok()?))
    }
}

impl Convert<FixedU128, Balance> for FixedU128Convert {
    fn convert(a: FixedU128) -> Balance {
        let accuracy = FixedU128::accuracy() / FixedI64::accuracy() as u128;
        // NOTE: unwrap is for testing purposes only. Do not use it in production.
        (a.into_inner() / accuracy).try_into().unwrap()
    }
}

use crate::PoolId;
use std::cell::RefCell;
use std::collections::HashMap;

pub struct Asset {
    total: Balance,
    balances: HashMap<AccountId, Balance>,
}

thread_local! {
    static ASSETS: RefCell<Vec<Asset>> = RefCell::new(Vec::new());
}

pub struct TestAssets;

impl TestAssets {
    // general method for assets creation
    pub fn create_asset() -> Result<AssetId, DispatchError> {
        ASSETS.with(|d| -> Result<AssetId, DispatchError> {
            let mut d = d.borrow_mut();
            let id =
                AssetId::try_from(d.len()).map_err(|_| DispatchError::Other(&"Too large id"))?;
            d.push(Asset {
                total: 0,
                balances: HashMap::new(),
            });

            Ok(id)
        })
    }
}

impl curve_amm::traits::Assets<AssetId, Balance, AccountId> for TestAssets {
    fn create_asset(_pool_id: crate::PoolId) -> Result<AssetId, DispatchError> {
        Self::create_asset()
    }

    fn mint(asset: AssetId, dest: &AccountId, amount: Balance) -> DispatchResult {
        ASSETS.with(|d| -> DispatchResult {
            let i =
                usize::try_from(asset).map_err(|_| DispatchError::Other(&"Index out of range"))?;
            let mut d = d.borrow_mut();
            let a = d
                .get_mut(i)
                .ok_or(DispatchError::Other(&"Index out of range"))?;

            if let Some(x) = a.balances.get_mut(dest) {
                *x = x
                    .checked_add(amount)
                    .ok_or(DispatchError::Other(&"Overflow"))?;
            } else {
                a.balances.insert(*dest, amount);
            }

            a.total = a
                .total
                .checked_add(amount)
                .ok_or(DispatchError::Other(&"Overflow"))?;

            Ok(())
        })
    }

    fn burn(asset: AssetId, dest: &AccountId, amount: Balance) -> DispatchResult {
        ASSETS.with(|d| -> DispatchResult {
            let i =
                usize::try_from(asset).map_err(|_| DispatchError::Other(&"Index out of range"))?;
            let mut d = d.borrow_mut();
            let a = d
                .get_mut(i)
                .ok_or(DispatchError::Other(&"Index out of range"))?;

            let x = a
                .balances
                .get_mut(dest)
                .ok_or(DispatchError::Other(&"Not found"))?;

            *x = x
                .checked_sub(amount)
                .ok_or(DispatchError::Other(&"Overflow"))?;

            a.total = a
                .total
                .checked_sub(amount)
                .ok_or(DispatchError::Other(&"Overflow"))?;

            Ok(())
        })
    }

    fn transfer(
        asset: AssetId,
        source: &AccountId,
        dest: &AccountId,
        amount: Balance,
    ) -> DispatchResult {
        Self::burn(asset, source, amount)?;
        Self::mint(asset, dest, amount)?;
        Ok(())
    }

    fn balance(asset: AssetId, who: &AccountId) -> Balance {
        ASSETS
            .with(|d| -> Option<Balance> {
                let i = usize::try_from(asset).ok()?;
                let d = d.borrow();
                let a = d.get(i)?;
                a.balances.get(who).map(|x| *x)
            })
            .unwrap_or(0)
    }

    fn total_issuance(asset: AssetId) -> Balance {
        ASSETS
            .with(|d| -> Option<Balance> {
                let i = usize::try_from(asset).ok()?;
                let d = d.borrow();
                d.get(i).map(|a| a.total)
            })
            .unwrap_or(0)
    }

    fn withdraw_admin_fees(
        pool_id: PoolId,
        amounts: impl Iterator<Item = Balance>,
    ) -> DispatchResult {
        let pool = CurveAmm::pool(pool_id).ok_or(DispatchError::Other(&"Pool not found"))?;
        let assets = pool.assets;

        for (asset, amount) in assets.into_iter().zip(amounts) {
            Self::transfer(
                asset,
                &CurveAmmModuleId::get().into_account_truncating(),
                &CURVE_ADMIN_FEE_ACC_ID,
                amount,
            )?;
        }

        Ok(())
    }
}

pub const CURVE_ADMIN_FEE_ACC_ID: AccountId = 343532;

pub struct EmptyUnbalanceHandler;

type Imbalance = <pallet_balances::Pallet<Test> as Currency<AccountId>>::NegativeImbalance;

impl OnUnbalanced<Imbalance> for EmptyUnbalanceHandler {}

thread_local! {
    static ON_POOL_CREATED_CALLED: RefCell<HashMap<PoolId, u32>> = RefCell::new(HashMap::new());
}

pub fn get_on_pool_created_called() -> HashMap<PoolId, u32> {
    ON_POOL_CREATED_CALLED.with(|v| v.borrow().clone())
}

pub struct OnPoolCreated;
impl super::traits::OnPoolCreated for OnPoolCreated {
    fn on_pool_created(pool_id: PoolId) {
        ON_POOL_CREATED_CALLED.with(|pool_created_called| {
            let mut mut_pool_created_called = pool_created_called.borrow_mut();
            mut_pool_created_called
                .entry(pool_id)
                .and_modify(|v| *v = *v + 1u32)
                .or_insert(1);
        });
    }
}

impl curve_amm::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type AdminOrigin = system::EnsureRoot<Self::AccountId>;
    type AssetId = i64;
    type Balance = Balance;
    type Currency = Balances;
    type CreationFee = CreationFee;
    type Assets = TestAssets;
    type OnUnbalanced = EmptyUnbalanceHandler;
    type PalletId = CurveAmmModuleId;
    type AssetChecker = ();
    type OnPoolCreated = OnPoolCreated;

    type Number = Number;
    type Precision = Precision;
    type Convert = FixedU128Convert;
    type WeightInfo = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into()
}
