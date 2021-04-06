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
    use crate::traits::Assets;
    use crate::{mock::*, PoolId};
    use frame_support::{assert_ok, traits::Currency};
    use sp_runtime::traits::AccountIdConversion;
    use sp_runtime::Permill;
    use sp_runtime::{FixedPointNumber, FixedU128};

    pub const ALICE_ID: AccountId = 1;
    pub const BOB_ID: AccountId = 2;
    pub const CHARLIE_ID: AccountId = 3;

    pub const TEST_POOL_ID: PoolId = 0;

    pub const BALANCE_ONE: Balance = 1_000_000_000;

    struct AddInitialLiquidityAndMintBobContext {
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

    fn init_add_initial_liquidity_and_mint_bob() -> AddInitialLiquidityAndMintBobContext {
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

        AddInitialLiquidityAndMintBobContext {
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

    struct MintAliceContext {
        alice: AccountId,
        swap: AccountId,
        pool: PoolId,
        pool_token: AssetId,
        coins: Vec<AssetId>,
        n_coins: usize,
        initial_amounts: Vec<Balance>,
    }

    fn init_mint_alice() -> MintAliceContext {
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

        MintAliceContext {
            alice,
            swap,
            pool,
            pool_token,
            coins,
            n_coins,
            initial_amounts,
        }
    }

    struct AddInitialLiquidityContext {
        alice: AccountId,
        bob: AccountId,
        swap: AccountId,
        pool: PoolId,
        pool_token: AssetId,
        coins: Vec<AssetId>,
        n_coins: usize,
        base_amount: Balance,
        initial_amounts: Vec<Balance>,
    }

    fn init_add_initial_liquidity(fee: Permill, admin_fee: Permill) -> AddInitialLiquidityContext {
        let alice = ALICE_ID;
        let bob = BOB_ID;
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
            fee,
            admin_fee,
        ));

        let pool_token = 2;

        // add_initial_liquidity
        assert_ok!(CurveAmm::add_liquidity(
            Origin::signed(alice),
            pool,
            initial_amounts.clone(),
            0
        ));

        AddInitialLiquidityContext {
            alice,
            bob,
            swap,
            pool,
            pool_token,
            coins,
            n_coins,
            base_amount,
            initial_amounts,
        }
    }

    mod test_add_liquidity {
        use super::super::last_event;
        use super::*;
        use crate::traits::Assets;
        use crate::Error;
        use frame_support::assert_err_ignore_postinfo;
        use frame_support::assert_ok;
        use sp_runtime::traits::Saturating;
        use sp_runtime::FixedI64;
        use sp_runtime::FixedPointNumber;

        #[test]
        fn test_add_liquidity() {
            new_test_ext().execute_with(|| {
                let AddInitialLiquidityAndMintBobContext {
                    bob,
                    swap,
                    pool,
                    pool_token,
                    coins,
                    n_coins,
                    base_amount,
                    initial_amounts,
                    ..
                } = init_add_initial_liquidity_and_mint_bob();

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
                let AddInitialLiquidityAndMintBobContext {
                    bob,
                    pool,
                    pool_token,
                    coins,
                    n_coins,
                    ..
                } = init_add_initial_liquidity_and_mint_bob();

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
                            let AddInitialLiquidityAndMintBobContext {
                                bob,
                                swap,
                                pool,
                                pool_token,
                                coins,
                                base_amount,
                                initial_amounts,
                                ..
                            } = init_add_initial_liquidity_and_mint_bob();

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
                let AddInitialLiquidityAndMintBobContext {
                    charlie,
                    pool,
                    coins,
                    ..
                } = init_add_initial_liquidity_and_mint_bob();

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
                let AddInitialLiquidityAndMintBobContext {
                    bob,
                    pool,
                    coins,
                    n_coins,
                    ..
                } = init_add_initial_liquidity_and_mint_bob();

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
                let AddInitialLiquidityAndMintBobContext {
                    bob,
                    pool,
                    coins,
                    n_coins,
                    ..
                } = init_add_initial_liquidity_and_mint_bob();

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
                let AddInitialLiquidityAndMintBobContext {
                    bob,
                    pool,
                    pool_token,
                    initial_amounts,
                    ..
                } = init_add_initial_liquidity_and_mint_bob();

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
        use crate::Error;
        use frame_support::assert_err_ignore_postinfo;
        use frame_support::assert_ok;
        use sp_runtime::FixedI64;
        use sp_runtime::FixedPointNumber;

        macro_rules! initial_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let MintAliceContext {
                                alice,
                                swap,
                                pool,
                                pool_token,
                                coins,
                                n_coins,
                                initial_amounts,
                                ..
                            } = init_mint_alice();

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
                            let MintAliceContext {
                                alice,
                                pool,
                                coins,
                                ..
                            } = init_mint_alice();

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

    mod test_remove_liquidity {
        use super::super::last_event;
        use super::*;
        use crate::traits::Assets;
        use crate::Error;
        use frame_support::assert_err_ignore_postinfo;
        use frame_support::assert_ok;

        macro_rules! remove_liquidity_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                alice,
                                swap,
                                pool,
                                pool_token,
                                coins,
                                n_coins,
                                base_amount,
                                initial_amounts,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let min_amount = $value;

                            assert_ok!(CurveAmm::remove_liquidity(
                                Origin::signed(alice),
                                pool,
                                (n_coins as Balance) * BALANCE_ONE * base_amount,
                                initial_amounts.iter().map(|i| i * min_amount).collect()
                            ));

                            for (&coin, &amount) in coins.iter().zip(initial_amounts.iter()) {
                                assert_eq!(TestAssets::balance(coin, &alice), amount);
                                assert_eq!(TestAssets::balance(coin, &swap), 0);
                            }

                            assert_eq!(
                                TestAssets::balance(pool_token, &alice),
                                0
                            );
                            assert_eq!(
                                TestAssets::total_issuance(pool_token),
                                0
                            );
                        });
                    }
                )*
            }
        }

        remove_liquidity_tests! {
            test_remove_liquidity_0: 0,
            test_remove_liquidity_1: 1,
        }

        #[test]
        fn test_remove_partial() {
            new_test_ext().execute_with(|| {
                let AddInitialLiquidityContext {
                    alice,
                    swap,
                    pool,
                    pool_token,
                    coins,
                    n_coins,
                    base_amount,
                    initial_amounts,
                    ..
                } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                let withdraw_amount = initial_amounts.iter().sum::<Balance>() / 2;

                assert_ok!(CurveAmm::remove_liquidity(
                    Origin::signed(alice),
                    pool,
                    withdraw_amount,
                    vec![0; n_coins]
                ));

                for (&coin, &amount) in coins.iter().zip(initial_amounts.iter()) {
                    let pool_balance = TestAssets::balance(coin, &swap);
                    let alice_balance = TestAssets::balance(coin, &alice);
                    assert_eq!(alice_balance + pool_balance, amount);
                }

                assert_eq!(
                    TestAssets::balance(pool_token, &alice),
                    (n_coins as Balance) * BALANCE_ONE * base_amount - withdraw_amount
                );
                assert_eq!(
                    TestAssets::total_issuance(pool_token),
                    (n_coins as Balance) * BALANCE_ONE * base_amount - withdraw_amount
                );
            });
        }

        macro_rules! below_min_amount_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                alice,
                                pool,
                                n_coins,
                                base_amount,
                                initial_amounts,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let idx = $value;

                            let mut min_amount = initial_amounts.clone();
                            min_amount[idx] = min_amount[idx] + 1;

                            assert_err_ignore_postinfo!(
                                CurveAmm::remove_liquidity(
                                    Origin::signed(alice),
                                    pool,
                                    (n_coins as Balance) * BALANCE_ONE * base_amount,
                                    min_amount,
                                ),
                                Error::<Test>::RequiredAmountNotReached
                            );
                        });
                    }
                )*
            }
        }

        below_min_amount_tests! {
            test_below_min_amount_0: 0,
            test_below_min_amount_1: 1,
        }

        #[test]
        fn test_amount_exceeds_balance() {
            new_test_ext().execute_with(|| {
                let AddInitialLiquidityContext {
                    alice,
                    pool,
                    n_coins,
                    base_amount,
                    ..
                } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                assert_err_ignore_postinfo!(
                    CurveAmm::remove_liquidity(
                        Origin::signed(alice),
                        pool,
                        (n_coins as Balance) * BALANCE_ONE * base_amount + 1,
                        vec![0; n_coins],
                    ),
                    Error::<Test>::InsufficientFunds
                );
            });
        }

        #[test]
        fn test_event() {
            new_test_ext().execute_with(|| {
                let AddInitialLiquidityContext {
                    alice,
                    bob,
                    pool,
                    pool_token,
                    coins,
                    n_coins,
                    ..
                } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                let _ = TestAssets::transfer(pool_token, &alice, &bob, BALANCE_ONE);

                System::set_block_number(2);

                assert_ok!(CurveAmm::remove_liquidity(
                    Origin::signed(bob),
                    pool,
                    BALANCE_ONE,
                    vec![0; n_coins],
                ));

                if let Event::curve_amm(crate::pallet::Event::RemoveLiquidity(
                    provider,
                    _,
                    token_amounts,
                    _,
                    token_supply,
                )) = last_event()
                {
                    assert_eq!(provider, bob);
                    assert_eq!(token_supply, TestAssets::total_issuance(pool_token));

                    for (&coin, &amount) in coins.iter().zip(token_amounts.iter()) {
                        assert_eq!(TestAssets::balance(coin, &bob), amount);
                    }
                } else {
                    panic!("Unexpected event");
                }
            });
        }
    }

    mod test_remove_liquidity_imbalance {
        use super::super::last_event;
        use super::*;
        use crate::traits::Assets;
        use crate::Error;
        use frame_support::assert_err_ignore_postinfo;
        use frame_support::assert_ok;

        macro_rules! remove_balanced_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                alice,
                                swap,
                                pool,
                                pool_token,
                                coins,
                                n_coins,
                                base_amount,
                                initial_amounts,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let divisor = $value;

                            let amounts = initial_amounts.iter().map(|i| i / divisor).collect::<Vec<_>>();
                            let max_burn = (n_coins as Balance) * BALANCE_ONE * base_amount / divisor;

                            assert_ok!(CurveAmm::remove_liquidity_imbalance(
                                Origin::signed(alice),
                                pool,
                                amounts.clone(),
                                max_burn + 1,
                            ));

                            for ((&coin, &amount), &initial) in coins.iter().zip(amounts.iter()).zip(initial_amounts.iter()) {
                                assert_eq!(TestAssets::balance(coin, &alice), amount);
                                assert_eq!(TestAssets::balance(coin, &swap), initial - amount);
                            }

                            assert!(
                                i128::abs(TestAssets::balance(pool_token, &alice) as i128 - ((n_coins as Balance) * BALANCE_ONE * base_amount - max_burn) as i128) <= 1
                            );
                            assert!(
                                i128::abs(TestAssets::total_issuance(pool_token) as i128 - ((n_coins as Balance) * BALANCE_ONE * base_amount - max_burn) as i128) <= 1
                            );
                        });
                    }
                )*
            }
        }

        remove_balanced_tests! {
            test_remove_balanced_2: 2,
            test_remove_balanced_5: 5,
            test_remove_balanced_10: 10,
        }

        macro_rules! remove_some_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                alice,
                                swap,
                                pool,
                                pool_token,
                                coins,
                                n_coins,
                                base_amount,
                                initial_amounts,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let idx = $value;

                            let mut amounts = initial_amounts.iter().map(|i| i / 2).collect::<Vec<_>>();
                            amounts[idx] = 0;

                            assert_ok!(CurveAmm::remove_liquidity_imbalance(
                                            Origin::signed(alice),
                                            pool,
                                            amounts.clone(),
                                            (n_coins as Balance) * BALANCE_ONE * base_amount,
                                        ));

                            for ((&coin, &amount), &initial) in coins.iter().zip(amounts.iter()).zip(initial_amounts.iter()) {
                                assert_eq!(TestAssets::balance(coin, &alice), amount);
                                assert_eq!(TestAssets::balance(coin, &swap), initial - amount);
                            }

                            let actual_balance = TestAssets::balance(pool_token, &alice);
                            let actual_total_supply = TestAssets::total_issuance(pool_token);
                            let ideal_balance = BALANCE_ONE * base_amount * (n_coins as Balance) - BALANCE_ONE * base_amount / 2 * ((n_coins as Balance) - 1);

                            assert_eq!(actual_balance, actual_total_supply);
                            assert!(ideal_balance * 99 / 100 < actual_balance);
                            assert!(actual_balance < ideal_balance);
                        });
                    }
                )*
            }
        }

        remove_some_tests! {
            test_remove_some_0: 0,
            test_remove_some_1: 1,
        }

        macro_rules! remove_one_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                alice,
                                swap,
                                pool,
                                pool_token,
                                coins,
                                n_coins,
                                base_amount,
                                initial_amounts,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let idx = $value;

                            let mut amounts = vec![0; n_coins];
                            amounts[idx] = initial_amounts[idx] / 2;

                            assert_ok!(CurveAmm::remove_liquidity_imbalance(
                                            Origin::signed(alice),
                                            pool,
                                            amounts.clone(),
                                            (n_coins as Balance) * BALANCE_ONE * base_amount,
                                        ));

                            for ((&coin, &amount), &initial) in coins.iter().zip(amounts.iter()).zip(initial_amounts.iter()) {
                                assert_eq!(TestAssets::balance(coin, &alice), amount);
                                assert_eq!(TestAssets::balance(coin, &swap), initial - amount);
                            }

                            let actual_balance = TestAssets::balance(pool_token, &alice);
                            let actual_total_supply = TestAssets::total_issuance(pool_token);
                            let ideal_balance = BALANCE_ONE * base_amount * (n_coins as Balance) - BALANCE_ONE * base_amount / 2;

                            assert_eq!(actual_balance, actual_total_supply);
                            assert!(ideal_balance * 99 / 100 < actual_balance);
                            assert!(actual_balance < ideal_balance);
                        });
                    }
                )*
            }
        }

        remove_one_tests! {
            test_remove_one_0: 0,
            test_remove_one_1: 1,
        }

        macro_rules! exceed_max_burn_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                alice,
                                pool,
                                n_coins,
                                base_amount,
                                initial_amounts,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let divisor = $value;

                            let amounts = initial_amounts.iter().map(|i| i / divisor).collect::<Vec<_>>();
                            let max_burn = (n_coins as Balance) * BALANCE_ONE * base_amount / divisor;

                            assert_err_ignore_postinfo!(
                                CurveAmm::remove_liquidity_imbalance(
                                    Origin::signed(alice),
                                    pool,
                                    amounts.clone(),
                                    max_burn - 1,
                                ),
                                Error::<Test>::RequiredAmountNotReached
                            );
                        });
                    }
                )*
            }
        }

        exceed_max_burn_tests! {
            test_exceed_max_burn_1: 1,
            test_exceed_max_burn_2: 2,
            test_exceed_max_burn_10: 10,
        }

        #[test]
        fn test_cannot_remove_zero() {
            new_test_ext().execute_with(|| {
                let AddInitialLiquidityContext {
                    alice,
                    pool,
                    n_coins,
                    ..
                } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                assert_err_ignore_postinfo!(
                    CurveAmm::remove_liquidity_imbalance(
                        Origin::signed(alice),
                        pool,
                        vec![0; n_coins],
                        0,
                    ),
                    Error::<Test>::WrongAssetAmount
                );
            });
        }

        #[test]
        fn test_no_total_supply() {
            new_test_ext().execute_with(|| {
                let AddInitialLiquidityContext {
                    alice,
                    pool,
                    pool_token,
                    n_coins,
                    ..
                } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                assert_ok!(CurveAmm::remove_liquidity(
                    Origin::signed(alice),
                    pool,
                    TestAssets::total_issuance(pool_token),
                    vec![0; n_coins],
                ));

                assert_err_ignore_postinfo!(
                    CurveAmm::remove_liquidity_imbalance(
                        Origin::signed(alice),
                        pool,
                        vec![0; n_coins],
                        0,
                    ),
                    Error::<Test>::InsufficientFunds
                );
            });
        }

        #[test]
        fn test_event() {
            new_test_ext().execute_with(|| {
                let AddInitialLiquidityContext {
                    alice,
                    bob,
                    pool,
                    pool_token,
                    coins,
                    n_coins,
                    base_amount,
                    initial_amounts,
                    ..
                } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                let _ = TestAssets::transfer(
                    pool_token,
                    &alice,
                    &bob,
                    TestAssets::balance(pool_token, &alice),
                );

                System::set_block_number(2);

                let amounts = initial_amounts.iter().map(|i| i / 5).collect::<Vec<_>>();
                let max_burn = (n_coins as Balance) * BALANCE_ONE * base_amount;

                assert_ok!(CurveAmm::remove_liquidity_imbalance(
                    Origin::signed(bob),
                    pool,
                    amounts,
                    max_burn,
                ));

                if let Event::curve_amm(crate::pallet::Event::RemoveLiquidityImbalance(
                    provider,
                    _,
                    token_amounts,
                    _,
                    _,
                    token_supply,
                )) = last_event()
                {
                    assert_eq!(provider, bob);
                    assert_eq!(token_supply, TestAssets::total_issuance(pool_token));

                    for (&coin, &amount) in coins.iter().zip(token_amounts.iter()) {
                        assert_eq!(TestAssets::balance(coin, &bob), amount);
                    }
                } else {
                    panic!("Unexpected event");
                }
            });
        }
    }

    mod test_remove_liquidity_one_coin {
        use super::super::last_event;
        use super::*;
        use crate::traits::Assets;
        use crate::{Error, PoolTokenIndex};
        use frame_support::assert_err_ignore_postinfo;
        use frame_support::assert_ok;

        macro_rules! amount_received_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                alice,
                                pool,
                                coins,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let idx: usize = $value;

                            let rate_mod = (100_001, 100_000);

                            assert_ok!(CurveAmm::remove_liquidity_one_coin(
                                            Origin::signed(alice),
                                            pool,
                                            BALANCE_ONE, idx as PoolTokenIndex, 0,
                                        ));

                            let balance = TestAssets::balance(coins[idx], &alice);

                            assert!(BALANCE_ONE * rate_mod.1 / rate_mod.0 <= balance);
                            assert!(balance <= BALANCE_ONE);
                        });
                    }
                )*
            }
        }

        amount_received_tests! {
            test_amount_received_0: 0,
            test_amount_received_1: 1,
        }

        macro_rules! lp_token_balance_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                alice,
                                pool,
                                pool_token,
                                n_coins,
                                base_amount,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let (idx, divisor) = $value;

                            let amount = TestAssets::balance(pool_token, &alice) / divisor;

                            assert_ok!(CurveAmm::remove_liquidity_one_coin(
                                Origin::signed(alice),
                                pool,
                                amount,
                                idx as PoolTokenIndex,
                                0,
                            ));

                            assert_eq!(
                                TestAssets::balance(pool_token, &alice),
                                (n_coins as Balance) * BALANCE_ONE * base_amount - amount
                            );
                        });
                    }
                )*
            }
        }

        lp_token_balance_tests! {
            test_lp_token_balance_0_1: (0, 1),
            test_lp_token_balance_0_5: (0, 5),
            test_lp_token_balance_0_42: (0, 42),
            test_lp_token_balance_1_1: (1, 1),
            test_lp_token_balance_1_5: (1, 5),
            test_lp_token_balance_1_42: (1, 42),
        }

        macro_rules! expected_vs_actual_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                alice,
                                pool,
                                pool_token,
                                coins,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let idx = $value;

                            let amount = TestAssets::balance(pool_token, &alice) / 10;

                            let expected = crate::Pallet::<Test>::get_withdraw_one_coin(pool, amount, idx as PoolTokenIndex).unwrap();

                            assert_ok!(CurveAmm::remove_liquidity_one_coin(
                                            Origin::signed(alice),
                                            pool,
                                            amount,
                                            idx as PoolTokenIndex,
                                            0,
                                        ));

                            assert_eq!(
                                TestAssets::balance(coins[idx], &alice),
                                expected
                            );
                        });
                    }
                )*
            }
        }

        expected_vs_actual_tests! {
            test_expected_vs_actual_0: 0,
            test_expected_vs_actual_1: 1,
        }

        macro_rules! below_min_amount_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                alice,
                                pool,
                                pool_token,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let idx = $value;

                            let amount = TestAssets::balance(pool_token, &alice);
                            let expected = crate::Pallet::<Test>::get_withdraw_one_coin(pool, amount, idx as PoolTokenIndex).unwrap();

                            assert_err_ignore_postinfo!(
                                CurveAmm::remove_liquidity_one_coin(Origin::signed(alice), pool, amount, idx, expected + 1),
                                Error::<Test>::RequiredAmountNotReached
                            );
                        });
                    }
                )*
            }
        }

        below_min_amount_tests! {
            test_below_min_amount_0: 0,
            test_below_min_amount_1: 1,
        }

        macro_rules! amount_exceeds_balance_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                bob,
                                pool,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let idx = $value;

                            assert_err_ignore_postinfo!(
                                            CurveAmm::remove_liquidity_one_coin(Origin::signed(bob), pool, 1, idx, 0),
                                            Error::<Test>::RequiredAmountNotReached
                                        );
                        });
                    }
                )*
            }
        }

        amount_exceeds_balance_tests! {
            test_amount_exceeds_balance_0: 0,
            test_amount_exceeds_balance_1: 1,
        }

        #[test]
        fn test_above_n_coins() {
            new_test_ext().execute_with(|| {
                let AddInitialLiquidityContext {
                    alice,
                    pool,
                    n_coins,
                    ..
                } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                assert_err_ignore_postinfo!(
                    CurveAmm::remove_liquidity_one_coin(
                        Origin::signed(alice),
                        pool,
                        1,
                        n_coins as PoolTokenIndex,
                        0
                    ),
                    Error::<Test>::Math
                );
            });
        }

        macro_rules! event_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            let AddInitialLiquidityContext {
                                alice,
                                bob,
                                pool,
                                pool_token,
                                coins,
                                ..
                            } = init_add_initial_liquidity(Permill::zero(), Permill::zero());

                            let idx = $value;

                            let _ = TestAssets::transfer(pool_token, &alice, &bob, BALANCE_ONE);

                            System::set_block_number(2);

                            assert_ok!(CurveAmm::remove_liquidity_one_coin(
                                Origin::signed(bob),
                                pool,
                                BALANCE_ONE,
                                idx as PoolTokenIndex,
                                0
                            ));

                            if let Event::curve_amm(crate::pallet::Event::RemoveLiquidityOne(
                                provider,
                                _,
                                token_amount,
                                coin_amount,
                                _,
                            )) = last_event()
                            {
                                assert_eq!(provider, bob);
                                assert_eq!(token_amount, BALANCE_ONE);
                                assert_eq!(coin_amount, TestAssets::balance(coins[idx], &bob));
                            } else {
                                panic!("Unexpected event");
                            }
                        });
                    }
                )*
            }
        }

        event_tests! {
            test_event_0: 0,
            test_event_1: 1,
        }
    }

    mod test_exchange {
        use super::super::last_event;
        use super::*;
        use crate::traits::Assets;
        use crate::{Error, PoolTokenIndex};
        use frame_support::assert_err_ignore_postinfo;
        use frame_support::assert_ok;
        use sp_runtime::traits::Saturating;
        use sp_runtime::FixedI64;
        use sp_runtime::FixedPointNumber;
        use sp_std::cmp::max;
        use sp_std::convert::TryFrom;

        fn get_admin_balances(swap: AccountId, coins: &[AssetId]) -> Vec<Balance> {
            let balances = CurveAmm::pools(0).unwrap().balances;
            coins
                .iter()
                .copied()
                .enumerate()
                .map(|(i, coin)| TestAssets::balance(coin, &swap) - balances[i])
                .collect()
        }

        fn approx(actual: FixedI64, expected: FixedI64, rel: FixedI64) -> bool {
            let delta = if expected > actual {
                expected - actual
            } else {
                actual - expected
            };

            delta / expected <= rel
        }

        macro_rules! exchange_tests {
            ($($name:ident: $value:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        new_test_ext().execute_with(|| {
                            const FEE_Q: u64 = 10_000;

                            let (sending, receiving, fee, admin_fee): (usize, usize, u64, u64) = $value;
                            let fee = Permill::from_rational_approximation(fee, FEE_Q);
                            let admin_fee = Permill::from_rational_approximation(admin_fee, FEE_Q);

                            let AddInitialLiquidityContext {
                                bob,
                                swap,
                                pool,
                                coins,
                                ..
                            } = init_add_initial_liquidity(fee, admin_fee);

                            let amount = BALANCE_ONE;
                            let _ = TestAssets::mint(coins[sending], &bob, amount);

                            assert_ok!(CurveAmm::exchange(
                                            Origin::signed(bob),
                                            pool,
                                            sending as PoolTokenIndex,
                                            receiving as PoolTokenIndex,
                                            amount,
                                            0
                                        ));

                            assert_eq!(TestAssets::balance(coins[sending], &bob), 0);

                            let received = TestAssets::balance(coins[receiving], &bob);
                            let received = FixedI64::from_inner(i64::try_from(received).unwrap());

                            let left_bound = FixedI64::one()
                                - max(
                                FixedI64::saturating_from_rational(1, 10_000),
                                FixedI64::one() / received,
                            )
                                - fee.into();

                            assert!(left_bound < received / 10.into());
                            assert!(received / 10.into() < FixedI64::one() - fee.into());

                            let expected_admin_fee = FixedI64::one() * fee.into() * admin_fee.into();
                            let admin_fees = get_admin_balances(swap, &coins);

                            if expected_admin_fee >= FixedI64::from_inner(1) {
                                assert!(approx(
                                    expected_admin_fee
                                        / FixedI64::from_inner(i64::try_from(admin_fees[receiving]).unwrap()),
                                    FixedI64::one(),
                                    max(
                                        FixedI64::saturating_from_rational(1, 1000),
                                        FixedI64::one()
                                            / (expected_admin_fee - FixedI64::saturating_from_rational(11, 10))
                                    )
                                ))
                            } else {
                                assert!(admin_fees[receiving] <= 1);
                            }
                        });
                    }
                )*
            }
        }

        exchange_tests! {
            test_exchange_0: (0, 1, 0, 0),
            test_exchange_1: (0, 1, 0, 400),
            test_exchange_2: (0, 1, 0, 1337),
            test_exchange_3: (0, 1, 0, 5000),
            test_exchange_4: (0, 1, 400, 400),
            test_exchange_5: (0, 1, 400, 1337),
            test_exchange_6: (0, 1, 400, 5000),
            test_exchange_7: (0, 1, 1337, 1337),
            test_exchange_8: (0, 1, 1337, 5000),
            test_exchange_9: (0, 1, 5000, 5000),
            test_exchange_10: (1, 0, 0, 0),
            test_exchange_11: (1, 0, 0, 400),
            test_exchange_12: (1, 0, 0, 1337),
            test_exchange_13: (1, 0, 0, 5000),
            test_exchange_14: (1, 0, 400, 400),
            test_exchange_15: (1, 0, 400, 1337),
            test_exchange_16: (1, 0, 400, 5000),
            test_exchange_17: (1, 0, 1337, 1337),
            test_exchange_18: (1, 0, 1337, 5000),
            test_exchange_19: (1, 0, 5000, 5000),
        }
    }
}
