use crate::{mock::*, Error, PoolInfo};
use core::convert::From;
use frame_support::assert_err_ignore_postinfo;
use frame_support::{assert_ok, traits::Currency};
use sp_runtime::traits::Saturating;
use sp_runtime::Permill;
use sp_runtime::{FixedPointNumber, FixedU128};
use sp_std::cmp::Ordering;

fn last_event() -> Event {
    frame_system::pallet::Pallet::<Test>::events()
        .pop()
        .expect("Event expected")
        .event
}

#[test]
fn create_pool_successful() {
    new_test_ext().execute_with(|| {
        let _ = Balances::deposit_creating(&1, 100000000);
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
        let _ = Balances::deposit_creating(&1, 100000000);

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
        let _ = Balances::deposit_creating(&1, 100000000);
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
        let _ = Balances::deposit_creating(&1, 100000000);
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
        let _ = Balances::deposit_creating(&1, initial_balance);
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
        let _ = Balances::deposit_creating(&1, initial_balance);
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

mod curve {
    use crate::{mock::*, PoolId};

    pub const ALICE_ID: AccountId = 1;
    pub const BOB_ID: AccountId = 2;
    pub const CHARLIE_ID: AccountId = 3;

    pub const TEST_POOL_ID: PoolId = 0;

    pub const BALANCE_ONE: Balance = 1_000_000_000;

    mod test_add_liquidity {
        use super::super::last_event;
        use super::*;
        use crate::traits::Assets;
        use crate::{Error, PoolId};
        use frame_support::assert_err_ignore_postinfo;
        use frame_support::{assert_ok, traits::Currency};
        use sp_runtime::traits::{AccountIdConversion, Saturating};
        use sp_runtime::{FixedI64, Permill};
        use sp_runtime::{FixedPointNumber, FixedU128};

        struct AddLiquidityTestContext {
            bob: AccountId,
            charlie: AccountId,
            swap: AccountId,
            pool: PoolId,
            pool_token: AssetId,
            coins: Vec<AssetId>,
            n_coins: usize,
            base_amount: Balance,
            initial_amounts: Vec<Balance>,
        }

        fn init_add_liquidity_test() -> AddLiquidityTestContext {
            let alice = ALICE_ID;
            let bob = BOB_ID;
            let charlie = CHARLIE_ID;
            let swap: u64 = CurveAmmModuleId::get().into_account();

            let pool = TEST_POOL_ID;

            let base_eq_amount: Balance = 100_000_000;

            let base_amount: Balance = 1_000_000;

            // Create pool tokens
            let coin0 = TestAssets::create_asset().unwrap();
            let coin1 = TestAssets::create_asset().unwrap();

            assert_eq!(coin0, 0);
            assert_eq!(coin1, 1);

            let coins = vec![coin0, coin1];
            let n_coins = coins.len();

            let initial_amounts = coins
                .iter()
                .map(|_| base_amount * BALANCE_ONE)
                .collect::<Vec<_>>();

            // Mint Alice
            let _ = Balances::deposit_creating(&alice, base_eq_amount);

            for (&coin, &amount) in coins.iter().zip(initial_amounts.iter()) {
                assert_ok!(TestAssets::mint(coin, &alice, amount));
            }

            // Create pool
            assert_ok!(CurveAmm::create_pool(
                Origin::signed(alice),
                vec![coin0, coin1],
                FixedU128::saturating_from_integer(360),
                Permill::zero(),
                Permill::zero(),
            ));

            let pool_token = 2;

            // add_initial_liquidity
            assert_ok!(CurveAmm::add_liquidity(
                Origin::signed(alice),
                pool,
                initial_amounts.clone(),
                0
            ));

            // mint_bob
            let _ = Balances::deposit_creating(&bob, base_eq_amount);

            for (&coin, &amount) in coins.iter().zip(initial_amounts.iter()) {
                assert_ok!(TestAssets::mint(coin, &bob, amount));
            }

            AddLiquidityTestContext {
                bob,
                charlie,
                swap,
                pool,
                pool_token,
                coins,
                n_coins,
                base_amount,
                initial_amounts,
            }
        }

        #[test]
        fn test_add_liquidity() {
            new_test_ext().execute_with(|| {
                let AddLiquidityTestContext {
                    bob,
                    swap,
                    pool,
                    pool_token,
                    coins,
                    n_coins,
                    base_amount,
                    initial_amounts,
                    ..
                } = init_add_liquidity_test();

                assert_ok!(CurveAmm::add_liquidity(
                    Origin::signed(bob),
                    pool,
                    initial_amounts.clone(),
                    0
                ));

                for (&coin, &amount) in coins.iter().zip(initial_amounts.iter()) {
                    assert_eq!(TestAssets::balance(coin, &bob), 0);
                    assert_eq!(TestAssets::balance(coin, &swap), 2 * amount);
                }

                assert_eq!(
                    TestAssets::balance(pool_token, &bob),
                    (n_coins as Balance) * BALANCE_ONE * base_amount
                );
                assert_eq!(
                    TestAssets::total_issuance(pool_token),
                    2 * (n_coins as Balance) * BALANCE_ONE * base_amount
                );
            });
        }

        #[test]
        fn test_add_with_slippage() {
            new_test_ext().execute_with(|| {
                let AddLiquidityTestContext {
                    bob,
                    pool,
                    pool_token,
                    coins,
                    n_coins,
                    ..
                } = init_add_liquidity_test();

                let mut amounts = coins.iter().map(|_| FixedI64::one()).collect::<Vec<_>>();
                amounts[0] = amounts[0].saturating_mul(FixedI64::saturating_from_rational(99, 100));
                amounts[1] =
                    amounts[1].saturating_mul(FixedI64::saturating_from_rational(101, 100));
                let amounts = amounts
                    .iter()
                    .map(|x| x.into_inner() as Balance)
                    .collect::<Vec<_>>();

                assert_ok!(CurveAmm::add_liquidity(
                    Origin::signed(bob),
                    pool,
                    amounts,
                    0
                ));

                let b = TestAssets::balance(pool_token, &bob) / n_coins as u64;

                assert!(
                    (FixedI64::saturating_from_rational(999, 1000).into_inner() as Balance) < b
                        && (b < FixedI64::one().into_inner() as Balance)
                );
            });
        }

        macro_rules! add_one_coin_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddLiquidityTestContext {
                                bob,
                                swap,
                                pool,
                                pool_token,
                                coins,
                                base_amount,
                                initial_amounts,
                                ..
                            } = init_add_liquidity_test();

                            let idx = $value;

                            let mut amounts = coins.iter().map(|_| 0 as Balance).collect::<Vec<_>>();
                            amounts[idx] = initial_amounts[idx];

                            assert_ok!(CurveAmm::add_liquidity(
                                Origin::signed(bob),
                                pool,
                                amounts.clone(),
                                0
                            ));

                            for (i, &coin) in coins.iter().enumerate() {
                                assert_eq!(
                                    TestAssets::balance(coin, &bob),
                                    initial_amounts[i] - amounts[i]
                                );
                                assert_eq!(
                                    TestAssets::balance(coin, &swap),
                                    initial_amounts[i] + amounts[i]
                                );
                            }

                            let b = TestAssets::balance(pool_token, &bob) / base_amount;

                            assert!(
                                (FixedI64::saturating_from_rational(999, 1000).into_inner() as Balance) < b
                                    && (b < FixedI64::one().into_inner() as Balance)
                            );
                        });
                    }
                )*
            }
        }

        add_one_coin_tests! {
            test_add_one_coin_0: 0,
            test_add_one_coin_1: 1,
        }

        #[test]
        fn test_insufficient_balance() {
            new_test_ext().execute_with(|| {
                let AddLiquidityTestContext {
                    charlie,
                    pool,
                    coins,
                    ..
                } = init_add_liquidity_test();

                let amounts = coins
                    .iter()
                    .map(|_| FixedI64::one().into_inner() as Balance)
                    .collect::<Vec<_>>();

                assert_err_ignore_postinfo!(
                    CurveAmm::add_liquidity(Origin::signed(charlie), pool, amounts, 0),
                    Error::<Test>::InsufficientFunds
                );
            });
        }

        #[test]
        fn test_min_amount_too_high() {
            new_test_ext().execute_with(|| {
                let AddLiquidityTestContext {
                    bob,
                    pool,
                    coins,
                    n_coins,
                    ..
                } = init_add_liquidity_test();

                let amounts = coins
                    .iter()
                    .map(|_| FixedI64::one().into_inner() as Balance)
                    .collect::<Vec<_>>();

                let min_amount = (FixedI64::one().into_inner() as Balance) * n_coins as Balance + 1;

                assert_err_ignore_postinfo!(
                    CurveAmm::add_liquidity(Origin::signed(bob), pool, amounts, min_amount),
                    Error::<Test>::RequiredAmountNotReached
                );
            });
        }

        #[test]
        fn test_min_amount_with_slippage() {
            new_test_ext().execute_with(|| {
                let AddLiquidityTestContext {
                    bob,
                    pool,
                    coins,
                    n_coins,
                    ..
                } = init_add_liquidity_test();

                let mut amounts = coins.iter().map(|_| FixedI64::one()).collect::<Vec<_>>();
                amounts[0] = amounts[0].saturating_mul(FixedI64::saturating_from_rational(99, 100));
                amounts[1] =
                    amounts[1].saturating_mul(FixedI64::saturating_from_rational(101, 100));
                let amounts = amounts
                    .iter()
                    .map(|x| x.into_inner() as Balance)
                    .collect::<Vec<_>>();

                assert_err_ignore_postinfo!(
                    CurveAmm::add_liquidity(
                        Origin::signed(bob),
                        pool,
                        amounts,
                        FixedI64::saturating_from_integer(n_coins as i64).into_inner() as Balance
                    ),
                    Error::<Test>::RequiredAmountNotReached
                );
            });
        }

        #[test]
        fn test_event() {
            new_test_ext().execute_with(|| {
                let AddLiquidityTestContext {
                    bob,
                    pool,
                    pool_token,
                    initial_amounts,
                    ..
                } = init_add_liquidity_test();

                System::set_block_number(2);

                assert_ok!(CurveAmm::add_liquidity(
                    Origin::signed(bob),
                    pool,
                    initial_amounts.clone(),
                    0
                ));

                if let Event::curve_amm(crate::pallet::Event::AddLiquidity(
                    provider,
                    _,
                    token_amounts,
                    _,
                    _,
                    token_supply,
                )) = last_event()
                {
                    assert_eq!(provider, bob);
                    assert_eq!(token_amounts, initial_amounts);
                    assert_eq!(token_supply, TestAssets::total_issuance(pool_token))
                } else {
                    panic!("Unexpected event");
                }
            });
        }
    }

    mod test_add_liquidity_initial {
        use super::*;
        use crate::traits::Assets;
        use crate::{Error, PoolId};
        use frame_support::assert_err_ignore_postinfo;
        use frame_support::{assert_ok, traits::Currency};
        use sp_runtime::traits::AccountIdConversion;
        use sp_runtime::{FixedI64, Permill};
        use sp_runtime::{FixedPointNumber, FixedU128};

        struct AddLiquidityInitialTestContext {
            alice: AccountId,
            swap: AccountId,
            pool: PoolId,
            pool_token: AssetId,
            coins: Vec<AssetId>,
            n_coins: usize,
            initial_amounts: Vec<Balance>,
        }

        fn init_add_liquidity_initial_test() -> AddLiquidityInitialTestContext {
            let alice = ALICE_ID;
            let swap: u64 = CurveAmmModuleId::get().into_account();

            let pool = TEST_POOL_ID;

            let base_eq_amount: Balance = 100_000_000;

            let base_amount: Balance = 1_000_000;

            // Create pool tokens
            let coin0 = TestAssets::create_asset().unwrap();
            let coin1 = TestAssets::create_asset().unwrap();

            assert_eq!(coin0, 0);
            assert_eq!(coin1, 1);

            let coins = vec![coin0, coin1];
            let n_coins = coins.len();

            let initial_amounts = coins
                .iter()
                .map(|_| base_amount * BALANCE_ONE)
                .collect::<Vec<_>>();

            // Mint Alice
            let _ = Balances::deposit_creating(&alice, base_eq_amount);

            for (&coin, &amount) in coins.iter().zip(initial_amounts.iter()) {
                assert_ok!(TestAssets::mint(coin, &alice, amount));
            }

            // Create pool
            assert_ok!(CurveAmm::create_pool(
                Origin::signed(alice),
                vec![coin0, coin1],
                FixedU128::saturating_from_integer(360),
                Permill::zero(),
                Permill::zero(),
            ));

            let pool_token = 2;

            AddLiquidityInitialTestContext {
                alice,
                swap,
                pool,
                pool_token,
                coins,
                n_coins,
                initial_amounts,
            }
        }

        macro_rules! initial_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddLiquidityInitialTestContext {
                                alice,
                                swap,
                                pool,
                                pool_token,
                                coins,
                                n_coins,
                                initial_amounts,
                                ..
                            } = init_add_liquidity_initial_test();

                            let min_amount: Balance = $value;

                            let amounts = coins
                                .iter()
                                .map(|_| FixedI64::one().into_inner() as Balance)
                                .collect::<Vec<_>>();

                            assert_ok!(
                                CurveAmm::add_liquidity(Origin::signed(alice), pool, amounts.clone(), min_amount)
                            );

                            for ((&coin, &amount), &initial) in coins.iter().zip(amounts.iter()).zip(initial_amounts.iter()) {
                                assert_eq!(TestAssets::balance(coin, &alice), initial - amount);
                                assert_eq!(TestAssets::balance(coin, &swap), amount);
                            }

                            assert_eq!(
                                TestAssets::balance(pool_token, &alice),
                                (n_coins as Balance) * BALANCE_ONE
                            );
                            assert_eq!(
                                TestAssets::total_issuance(pool_token),
                                (n_coins as Balance) * BALANCE_ONE
                            );
                        });
                    }
                )*
            }
        }

        initial_tests! {
            test_initial_0: 0,
            test_initial_2: 2 * BALANCE_ONE,
        }

        macro_rules! initial_liquidity_missing_coin_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddLiquidityInitialTestContext {
                                alice,
                                pool,
                                coins,
                                ..
                            } = init_add_liquidity_initial_test();

                            let idx = $value;

                            let mut amounts = coins
                                .iter()
                                .map(|_| FixedI64::one().into_inner() as Balance)
                                .collect::<Vec<_>>();
                            amounts[idx] = 0;

                            assert_err_ignore_postinfo!(
                                CurveAmm::add_liquidity(Origin::signed(alice), pool, amounts, 0),
                                Error::<Test>::WrongAssetAmount
                            );
                        });
                    }
                )*
            }
        }

        initial_liquidity_missing_coin_tests! {
            test_initial_liquidity_missing_coin_0: 0,
            test_initial_liquidity_missing_coin_1: 1,
        }
    }
}
