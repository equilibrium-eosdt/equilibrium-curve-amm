#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{account, benchmarks};
use frame_system::{pallet_prelude::OriginFor, Module as System, RawOrigin};
use sp_runtime::Permill;

use crate::Pallet as Curve;

const SEED: u32 = 0;

pub trait Config: crate::Config {}
pub struct Module<T: Config>(crate::Pallet<T>);

fn get_pool_params<T: Config>() -> (
    T::AccountId,
    Vec<<T as crate::Config>::AssetId>,
    Permill,
    Permill,
    T::Number,
) {
    let account = create_account::<T>("owner", 0, SEED, 1_000_000usize, None);
    let assets: Vec<<T as crate::Config>::AssetId> = (0..5)
        .into_iter()
        .map(|_| T::Assets::create_benchmark_asset())
        .collect();
    let fee = Permill::from_parts(600);
    let admin_fee = Permill::from_parts(200);
    let amplification = Curve::<T>::get_number(100);

    (account, assets, fee, admin_fee, amplification)
}

fn create_pool_<T: Config>() -> PoolId {
    let (owner, assets, fee, admin_fee, amplification) = get_pool_params::<T>();

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
    let depositor = create_account::<T>("depositor", 0, SEED, 1_000_000usize, Some(&pool.assets));
    let amounts: Vec<T::Balance> = pool
        .assets
        .iter()
        .map(|&a| convert_to_balance::<T>(amount as usize))
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

/// For using random_seed(){...} from
/// https://docs.rs/pallet-randomness-collective-flip/3.0.0/pallet_randomness_collective_flip/
fn wait_81_blocks<T: Config>() {
    System::<T>::set_block_number(81u32.into());
}

benchmarks! {
    create_pool{
        wait_81_blocks::<T>();

        let (owner, assets, fee, admin_fee, amplification) = get_pool_params::<T>();
        let pool_count = Curve::<T>::pool_count();

    }: _(RawOrigin::Signed(owner), assets, amplification, fee, admin_fee)
    verify {
        assert_eq!(pool_count+1, Curve::<T>::pool_count());
    }

    add_liquidity{
        wait_81_blocks::<T>();

        let pool_id = create_pool_::<T>();
        let pool = get_pool_info::<T>(pool_id);
        let depositor = create_account::<T>("depositor", 0, SEED, 1_000_000_000usize, Some(&pool.assets));
        let amounts: Vec<T::Balance> = pool.assets.iter()
            .map(|_| convert_to_balance::<T>(10_000 as usize))
            .collect();

        let min_mint_amount: T::Balance = convert_to_balance::<T>(1 as usize);

        let expected_total_balances = pool.total_balances
            .iter()
            .zip(amounts.iter())
            .map(|(&tb, &a)| tb + a)
            .collect::<Vec<T::Balance>>();

    }: _(RawOrigin::Signed(depositor), pool_id, amounts.clone(), min_mint_amount)
    verify {
        let pool = get_pool_info::<T>(pool_id);
        pool.balances.iter().zip(amounts.iter()).for_each(|(&b, &a)| assert_eq!(b, a));
    }

    exchange{
        wait_81_blocks::<T>();

        let pool_id = create_pool_::<T>();
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
        wait_81_blocks::<T>();

        let pool_id = create_pool_::<T>();
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
        wait_81_blocks::<T>();

        let pool_id = create_pool_::<T>();
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

        let pool_id = create_pool_::<T>();
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

        let pool_id = create_pool_::<T>();
        let depositor_1 = add_liquidity_::<T>(pool_id, 10_000u32);
        let i_1 = 1;
        let j_1 = 2;
        let amount_1 = convert_to_balance::<T>(100usize);
        let min_amount_to_receive_1 = convert_to_balance::<T>(0usize);
        let origin_1: OriginFor<T> = RawOrigin::Signed(depositor_1.clone()).into();

        let depositor_2 = add_liquidity_::<T>(pool_id, 10_000u32);
        let i_2 = 2;
        let j_2 = 3;
        let amount_2 = convert_to_balance::<T>(50usize);
        let min_amount_to_receive_2 = convert_to_balance::<T>(0usize);
        let origin_2: OriginFor<T> = RawOrigin::Signed(depositor_2).into();

        for i in 0..10{
            Curve::<T>::exchange(origin_1.clone(), pool_id, i_1, j_1, amount_1, min_amount_to_receive_1).unwrap();
            Curve::<T>::exchange(origin_2.clone(), pool_id, i_2, j_2, amount_2, min_amount_to_receive_2).unwrap();
        }

    }:  _(RawOrigin::Signed(depositor_1), pool_id)
    verify{
        let pool = get_pool_info::<T>(pool_id);
        pool.total_balances.iter().zip(pool.balances.iter()).for_each(|(&tb, &b)|{
            assert_eq!(tb, b);
        });
    }
}
