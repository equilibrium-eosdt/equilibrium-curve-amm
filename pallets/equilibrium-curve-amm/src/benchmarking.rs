#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::traits::BenchmarkingInit;
use frame_benchmarking::{account, benchmarks};
use frame_system::{pallet_prelude::OriginFor, Module as System, RawOrigin};
use sp_runtime::Permill;

use crate::Pallet as Curve;
use frame_support::PalletId;

const SEED: u32 = 0;
const CURVE_AMM_MODULE_ID: PalletId = PalletId(*b"eq/crvam");

pub trait Config: crate::Config {}
pub struct Pallet<T: Config>(crate::Pallet<T>);

fn get_pool_params<T: Config>(
    assets_count: u32,
) -> (
    T::AccountId,
    Vec<<T as crate::Config>::AssetId>,
    Permill,
    Permill,
    T::Number,
) {
    let account = create_account::<T>("owner", 0, SEED, 1_000_000_000usize, None);
    let assets: Vec<<T as crate::Config>::AssetId> = (0..assets_count)
        .into_iter()
        .map(|_| T::Assets::create_benchmark_asset())
        .collect();
    let fee = Permill::from_parts(600);
    let admin_fee = Permill::from_parts(200);
    let amplification = Curve::<T>::get_number(100);

    (account, assets, fee, admin_fee, amplification)
}

fn init_module_account<T: Config>() {
    <T::Currency as Currency<T::AccountId>>::make_free_balance_be(
        &CURVE_AMM_MODULE_ID.into_account_truncating(),
        convert_to_balance::<T>(1_000_000usize),
    );
}

fn create_pool_<T: Config>(assets_count: u32) -> PoolId {
    let (owner, assets, fee, admin_fee, amplification) = get_pool_params::<T>(assets_count);

    Curve::<T>::create_pool(
        RawOrigin::Signed(owner).into(),
        assets,
        amplification,
        fee,
        admin_fee,
    )
    .unwrap();

    Curve::<T>::pool_count() - 1
}

fn add_liquidity_<T: Config>(pool_id: PoolId, amount: u32) -> T::AccountId {
    let pool = Curve::<T>::pool(pool_id).unwrap();
    let depositor =
        create_account::<T>("depositor", 0, SEED, 1_000_000_000usize, Some(&pool.assets));
    let amounts: Vec<T::Balance> = pool
        .assets
        .iter()
        .map(|_| convert_to_balance::<T>(amount as usize))
        .collect();

    let min_mint_amount: T::Balance = convert_to_balance::<T>(1 as usize);

    Curve::<T>::add_liquidity(
        RawOrigin::Signed(depositor.clone()).into(),
        pool_id,
        amounts,
        min_mint_amount,
    )
    .unwrap();

    depositor
}

fn get_pool_info<T: crate::Config>(
    pool_id: PoolId,
) -> PoolInfo<T::AccountId, T::AssetId, T::Number, T::Balance> {
    Curve::<T>::pool(pool_id).unwrap()
}

fn create_account<T: Config>(
    name: &'static str,
    index: u32,
    seed: u32,
    initial_balance: usize,
    maybe_assets: Option<&Vec<T::AssetId>>,
) -> T::AccountId {
    let account = account(name, index, seed);

    let balance = convert_to_balance::<T>(initial_balance);
    <T::Currency as Currency<T::AccountId>>::make_free_balance_be(&account, balance);

    maybe_assets.map(|assets| {
        assets.iter().for_each(|&a| {
            T::Assets::mint(a, &account, balance).unwrap();
        })
    });

    account
}

fn convert_to_balance<T: Config>(number: usize) -> T::Balance {
    let number = <T::Convert as CheckedConvert<usize, T::Number>>::convert(number).unwrap();
    <T::Convert as Convert<T::Number, T::Balance>>::convert(number)
}

/// For using random_seed(){...} from pallet-randomness-collective-flip
/// https://docs.rs/pallet-randomness-collective-flip/3.0.0/pallet_randomness_collective_flip/
fn wait_81_blocks<T: Config>() {
    System::<T>::set_block_number(81u32.into());
}

benchmarks! {
    create_pool{
        let b in 2 .. 6;

        wait_81_blocks::<T>();
        let (owner, assets, fee, admin_fee, amplification) = get_pool_params::<T>(b);
        let pool_count = Curve::<T>::pool_count();

    }: _(RawOrigin::Signed(owner), assets, amplification, fee, admin_fee)
    verify {
        assert_eq!(pool_count+1, Curve::<T>::pool_count());
    }

    add_liquidity_without_fee{
        let b in 2 .. 6;

        wait_81_blocks::<T>();
        init_module_account::<T>();

        let pool_id = create_pool_::<T>(b);
        let pool = get_pool_info::<T>(pool_id);
        let depositor = create_account::<T>("depositor", 0, SEED, 1_000_000_000usize, Some(&pool.assets));
        let amounts: Vec<T::Balance> = pool.assets.iter()
            .map(|_| convert_to_balance::<T>(10_000 as usize))
            .collect();

        let min_mint_amount: T::Balance = convert_to_balance::<T>(1 as usize);

    }: add_liquidity(RawOrigin::Signed(depositor), pool_id, amounts.clone(), min_mint_amount)
    verify {
        let pool = get_pool_info::<T>(pool_id);
        pool.balances.iter().zip(amounts.iter()).for_each(|(&b, &a)| assert_eq!(b, a));
    }

    add_liquidity_with_fee{
        let b in 2 .. 6;

        wait_81_blocks::<T>();
        init_module_account::<T>();

        let pool_id = create_pool_::<T>(b);
        let depositor = add_liquidity_::<T>(pool_id, 10_000u32);
        let pool = get_pool_info::<T>(pool_id);
        let amounts: Vec<T::Balance> = pool.assets.iter()
            .map(|_| convert_to_balance::<T>(10_000 as usize))
            .collect();

        let expected_total_balances: Vec<T::Balance> = pool.total_balances
            .into_iter()
            .zip(amounts.clone())
            .map(|(tb, a)| tb + a)
            .collect();

        let min_mint_amount: T::Balance = convert_to_balance::<T>(1 as usize);

    }: add_liquidity(RawOrigin::Signed(depositor), pool_id, amounts.clone(), min_mint_amount)
    verify {
        let pool = get_pool_info::<T>(pool_id);
        pool.total_balances.iter()
            .zip(expected_total_balances.iter())
            .for_each(|(&tb, &eb)| assert_eq!(tb, eb));
    }

    exchange{
        wait_81_blocks::<T>();
        init_module_account::<T>();

        let assets_count = 5;
        let pool_id = create_pool_::<T>(assets_count);
        add_liquidity_::<T>(pool_id, 10_000u32);

        let pool = get_pool_info::<T>(pool_id);
        let exchanger = create_account::<T>("depositor", 0, SEED, 1_000_000usize, Some(&pool.assets));
        let send_token_index = 0usize;
        let received_token_index = 1usize;
        let amount = convert_to_balance::<T>(1000usize);
        let min_amount_to_receive = convert_to_balance::<T>(1usize);

    }: _(RawOrigin::Signed(exchanger), pool_id, send_token_index as u32, received_token_index as u32, amount, min_amount_to_receive)
    verify{
        let pool_after = get_pool_info::<T>(pool_id);
        assert_eq!(pool_after.total_balances[send_token_index], pool.total_balances[send_token_index].checked_add(&amount).unwrap());
        assert!(pool_after.total_balances[received_token_index] < pool.total_balances[received_token_index]);
    }

    remove_liquidity{
        let b in 2 .. 6;

        wait_81_blocks::<T>();
        init_module_account::<T>();

        let pool_id = create_pool_::<T>(b);
        let depositor = add_liquidity_::<T>(pool_id, 10_000u32);
        let pool = get_pool_info::<T>(pool_id);
        let amount_to_remove = T::Assets::balance(pool.pool_asset, &depositor);
        let min_amounts: Vec<T::Balance> = pool.assets.iter().map(|_| convert_to_balance::<T>(1usize)).collect();

    }: _(RawOrigin::Signed(depositor.clone()), pool_id, amount_to_remove, min_amounts)
    verify{
        let zero = convert_to_balance::<T>(0usize);
        assert_eq!(T::Assets::balance(pool.pool_asset, &depositor), zero);
    }

    remove_liquidity_imbalance{
        let b in 2 .. 6;

        wait_81_blocks::<T>();
        init_module_account::<T>();

        let pool_id = create_pool_::<T>(b);
        let depositor = add_liquidity_::<T>(pool_id, 10_000u32);
        let pool = get_pool_info::<T>(pool_id);
        let max_burn_amount = convert_to_balance::<T>(1_000usize);
        let amounts_to_remove:Vec<T::Balance> =  pool.assets.iter().map(|_| convert_to_balance::<T>(100usize)).collect();
        let balances_before_remove = pool.balances;

    }: _(RawOrigin::Signed(depositor), pool_id, amounts_to_remove.clone(), max_burn_amount)
    verify{
        let pool = get_pool_info::<T>(pool_id);
        pool.balances.iter()
            .zip(balances_before_remove.iter())
            .zip(amounts_to_remove.iter())
            .for_each(|((&b, &bb), ar)|{
               assert_eq!(bb, b.checked_add(ar).unwrap());
            });
    }

    remove_liquidity_one_coin{
        wait_81_blocks::<T>();
        init_module_account::<T>();

        let assets_count = 5;
        let pool_id = create_pool_::<T>(assets_count);
        let depositor = add_liquidity_::<T>(pool_id, 10_000u32);
        let remove_coin_index = 0usize;
        let pool = get_pool_info::<T>(pool_id);
        let amount_to_remove = convert_to_balance::<T>(1000usize);
        let min_amount = convert_to_balance::<T>(1usize);
        let balance_before_remove = T::Assets::balance(pool.pool_asset, &depositor);

    }: _(RawOrigin::Signed(depositor.clone()), pool_id, amount_to_remove, remove_coin_index as u32, min_amount)
    verify{
        let balance_after_remove = T::Assets::balance(pool.pool_asset, &depositor);
        assert_eq!(balance_after_remove.checked_add(&amount_to_remove).unwrap(), balance_before_remove);
    }

    withdraw_admin_fees{
        wait_81_blocks::<T>();
        init_module_account::<T>();

        let assets_count = 5;
        let pool_id = create_pool_::<T>(assets_count);
        let depositor = add_liquidity_::<T>(pool_id, 1000u32);
        let pool = get_pool_info::<T>(pool_id);

        let exchanger = create_account::<T>("exchanger", 0, SEED, 1_000_000usize, Some(&pool.assets));
        let i = 0;
        let j = 1;
        let amount = convert_to_balance::<T>(100usize);
        let min_amount_to_receive = convert_to_balance::<T>(1usize);
        let origin: OriginFor<T> = RawOrigin::Signed(exchanger).into();

        for _ in 0..10{
            Curve::<T>::exchange(origin.clone(), pool_id, i, j, amount, min_amount_to_receive).unwrap();
        }

        T::BenchmarkingInit::init_withdraw_admin_fees();
    }:  _(RawOrigin::Signed(depositor), pool_id)
    verify{
        let pool = get_pool_info::<T>(pool_id);
        pool.total_balances.iter().zip(pool.balances.iter()).for_each(|(&tb, &b)|{
            assert_eq!(tb, b);
        });
    }
}
