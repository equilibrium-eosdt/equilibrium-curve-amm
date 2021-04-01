use crate::{mock::*, Error, PoolId, PoolInfo};
use core::convert::From;
use frame_support::assert_err_ignore_postinfo;
use frame_support::{assert_noop, assert_ok, traits::Currency};
use sp_runtime::traits::{AccountIdConversion, Saturating};
use sp_runtime::Permill;
use sp_runtime::{FixedI64, FixedPointNumber, FixedU128};
use sp_std::cmp::Ordering;

#[test]
fn create_pool_successful() {
    new_test_ext().execute_with(|| {
        Balances::deposit_creating(&1, 100000000);
        assert_ok!(CurveAmm::create_pool(
            Origin::signed(1),
            vec![0, 1],
            FixedU128::from(1u128),
            Permill::one(),
            Permill::one(),
        ));
    });
}

#[test]
fn create_pool_assets_must_be_nonempty() {
    new_test_ext().execute_with(|| {
        Balances::deposit_creating(&1, 100000000);

        assert_err_ignore_postinfo!(
            CurveAmm::create_pool(
                Origin::signed(1),
                vec![],
                FixedU128::from(1u128),
                Permill::one(),
                Permill::one(),
            ),
            Error::<Test>::NotEnoughAssets
        );
    });
}

#[test]
fn create_pool_balance_must_be_more_than_fee() {
    new_test_ext().execute_with(|| {
        assert_err_ignore_postinfo!(
            CurveAmm::create_pool(
                Origin::signed(1),
                vec![0, 1],
                FixedU128::from(1u128),
                Permill::one(),
                Permill::one(),
            ),
            Error::<Test>::InsufficientFunds
        );
    });
}

#[test]
fn create_pool_correct_pool_count() {
    new_test_ext().execute_with(|| {
        assert_eq!(CurveAmm::pool_count(), 0);
        Balances::deposit_creating(&1, 100000000);
        assert_ok!(CurveAmm::create_pool(
            Origin::signed(1),
            vec![0, 1],
            FixedU128::from(1u128),
            Permill::one(),
            Permill::one(),
        ));
        assert_eq!(CurveAmm::pool_count(), 1);
    });
}

#[test]
fn create_pool_pool_saved_to_storage() {
    new_test_ext().execute_with(|| {
        assert_eq!(CurveAmm::pool_count(), 0);
        Balances::deposit_creating(&1, 100000000);
        assert_ok!(CurveAmm::create_pool(
            Origin::signed(1),
            vec![0, 1],
            FixedU128::from(1u128),
            Permill::one(),
            Permill::one(),
        ));
        assert_eq!(
            CurveAmm::pools(0),
            Some(PoolInfo {
                pool_asset: 0,
                assets: vec![0, 1],
                amplification: FixedU128::from(1u128),
                fee: Permill::one(),
                admin_fee: Permill::one(),
                balances: vec![0u64, 0u64,]
            })
        );
    });
}

#[test]
fn create_pool_fee_withdrawn() {
    new_test_ext().execute_with(|| {
        let initial_balance = 100000000;
        Balances::deposit_creating(&1, initial_balance);
        assert_ok!(CurveAmm::create_pool(
            Origin::signed(1),
            vec![0, 1],
            FixedU128::from(1u128),
            Permill::one(),
            Permill::one(),
        ));
        let balance_after_fee = Balances::free_balance(&1);
        assert_eq!(initial_balance - balance_after_fee, 999);
    });
}

#[test]
fn create_pool_on_unbalanced_called() {
    new_test_ext().execute_with(|| {
        let initial_balance = 100000000;
        Balances::deposit_creating(&1, initial_balance);
        assert_ok!(CurveAmm::create_pool(
            Origin::signed(1),
            vec![0, 1],
            FixedU128::from(1u128),
            Permill::one(),
            Permill::one(),
        ));
        let balance_after_fee = Balances::free_balance(&1);
        assert_eq!(initial_balance - balance_after_fee, 999);
    });
}

#[test]
fn get_d_successful() {
    let xp = vec![
        FixedU128::saturating_from_rational(11, 10),
        FixedU128::saturating_from_rational(88, 100),
    ];
    let amp = FixedU128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_d(&xp, ann);

    // expected d is 1.9781953712751776
    // expected precision is 1e-13
    let delta = result
        .map(|x| {
            x.saturating_sub(FixedU128::saturating_from_rational(
                19781953712751776u128,
                10_000_000_000_000_000u128,
            ))
            .saturating_abs()
        })
        .map(|x| {
            x.cmp(&FixedU128::saturating_from_rational(
                1u128,
                10_000_000_000_000u128,
            ))
        });
    assert_eq!(delta, Some(Ordering::Less));
}

#[test]
fn get_d_empty() {
    let xp = vec![];
    let amp = FixedU128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_d(&xp, ann);

    assert_eq!(result, Some(FixedU128::zero()));
}

#[test]
fn get_y_successful() {
    let i = 0;
    let j = 1;
    let x = FixedU128::saturating_from_rational(111, 100);
    let xp = vec![
        FixedU128::saturating_from_rational(11, 10),
        FixedU128::saturating_from_rational(88, 100),
    ];
    let amp = FixedU128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_y(i, j, x, &xp, ann);

    // expected y is 0.8703405416689252
    // expected precision is 1e-13
    let delta = result
        .map(|x| {
            x.saturating_sub(FixedU128::saturating_from_rational(
                8703405416689252u128,
                10_000_000_000_000_000u128,
            ))
            .saturating_abs()
        })
        .map(|x| {
            x.cmp(&FixedU128::saturating_from_rational(
                1,
                10_000_000_000_000u128,
            ))
        });
    assert_eq!(delta, Some(Ordering::Less));
}

#[test]
fn get_y_same_coin() {
    let i = 1;
    let j = 1;
    let x = FixedU128::saturating_from_rational(111, 100);
    let xp = vec![
        FixedU128::saturating_from_rational(11, 10),
        FixedU128::saturating_from_rational(88, 100),
    ];
    let amp = FixedU128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_y(i, j, x, &xp, ann);

    assert_eq!(result, None);
}

#[test]
fn get_y_i_greater_than_n() {
    let i = 33;
    let j = 1;
    let x = FixedU128::saturating_from_rational(111, 100);
    let xp = vec![
        FixedU128::saturating_from_rational(11, 10),
        FixedU128::saturating_from_rational(88, 100),
    ];
    let amp = FixedU128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_y(i, j, x, &xp, ann);

    assert_eq!(result, None);
}

#[test]
fn get_y_j_greater_than_n() {
    let i = 1;
    let j = 33;
    let x = FixedU128::saturating_from_rational(111, 100);
    let xp = vec![
        FixedU128::saturating_from_rational(11, 10),
        FixedU128::saturating_from_rational(88, 100),
    ];
    let amp = FixedU128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_y(i, j, x, &xp, ann);

    assert_eq!(result, None);
}

const AliceId: AccountId = 1;
const BobId: AccountId = 2;

const TestPoolId: PoolId = 0;

const BalanceOne: Balance = 1_000_000_000;

use crate::traits::Assets;

#[test]
fn test_add_liquidity() {
    new_test_ext().execute_with(|| {
        let alice = &AliceId;
        let bob = &BobId;
        let swap = &CurveAmmModuleId::get().into_account();

        let pool = TestPoolId;

        let base_eq_amount: Balance = 100_000_000;

        let base_amount: Balance = 1_000_000;

        // Create pool tokens
        let coin0 = TestAssets::create_asset().unwrap();
        let coin1 = TestAssets::create_asset().unwrap();

        assert_eq!(coin0, 0);
        assert_eq!(coin1, 1);

        let coins = [coin0, coin1];
        let n_coins = coins.len();

        let initial_amounts = coins
            .iter()
            .map(|_| base_amount * BalanceOne)
            .collect::<Vec<_>>();

        // Mint Alice
        Balances::deposit_creating(alice, base_eq_amount);

        for (&coin, &amount) in coins.iter().zip(initial_amounts.iter()) {
            assert_ok!(TestAssets::mint(coin, alice, amount));
        }

        // Create pool
        assert_ok!(CurveAmm::create_pool(
            Origin::signed(*alice),
            vec![coin0, coin1],
            FixedU128::from(1u128),
            Permill::one(),
            Permill::one(),
        ));

        let pool_token = 2;

        // add_initial_liquidity
        assert_ok!(CurveAmm::add_liquidity(
            Origin::signed(*alice),
            pool,
            initial_amounts.clone(),
            0
        ));

        // mint_bob
        Balances::deposit_creating(bob, base_eq_amount);

        for (&coin, &amount) in coins.iter().zip(initial_amounts.iter()) {
            assert_ok!(TestAssets::mint(coin, bob, amount));
        }

        // test_add_liquidity
        assert_ok!(CurveAmm::add_liquidity(
            Origin::signed(*bob),
            pool,
            initial_amounts.clone(),
            0
        ));

        for (&coin, &amount) in coins.iter().zip(initial_amounts.iter()) {
            assert_eq!(TestAssets::balance(coin, bob), 0);
            assert_eq!(TestAssets::balance(coin, swap), 2 * amount);
        }

        assert_eq!(
            TestAssets::balance(pool_token, bob),
            (n_coins as Balance) * BalanceOne * base_amount
        );
        assert_eq!(
            TestAssets::total_issuance(pool_token),
            2 * (n_coins as Balance) * BalanceOne * base_amount
        );
    });
}
