use crate::{mock::*, Error, PoolInfo};
use core::convert::From;
use frame_support::assert_err_ignore_postinfo;
use frame_support::{assert_noop, assert_ok, traits::Currency};
use sp_runtime::traits::Saturating;
use sp_runtime::Permill;
use sp_runtime::{FixedI128, FixedPointNumber};
use sp_std::cmp::Ordering;

#[test]
fn create_pool_successful() {
    new_test_ext().execute_with(|| {
        Balances::deposit_creating(&1, 100000000);
        assert_ok!(CurveAmm::create_pool(
            Origin::signed(1),
            vec![0, 1],
            FixedI128::from(1i128),
            Permill::one()
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
                FixedI128::from(1i128),
                Permill::one()
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
                FixedI128::from(1i128),
                Permill::one()
            ),
            Error::<Test>::NotEnoughForFee
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
            FixedI128::from(1i128),
            Permill::one()
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
            FixedI128::from(1i128),
            Permill::one()
        ));
        assert_eq!(
            CurveAmm::pools(0),
            Some(PoolInfo {
                pool_asset: 0,
                assets: vec![0, 1],
                amplification: FixedI128::from(1i128),
                fee: Permill::one()
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
            FixedI128::from(1i128),
            Permill::one()
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
            FixedI128::from(1i128),
            Permill::one()
        ));
        let balance_after_fee = Balances::free_balance(&1);
        assert_eq!(initial_balance - balance_after_fee, 999);
    });
}

#[test]
fn get_d_successful() {
    let xp = vec![
        FixedI128::saturating_from_rational(11, 10),
        FixedI128::saturating_from_rational(88, 100),
    ];
    let amp = FixedI128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_d(&xp, ann);

    // expected d is 1.9781953712751776
    // expected precision is 1e-13
    let delta = result
        .map(|x| {
            x.saturating_sub(FixedI128::saturating_from_rational(
                19781953712751776i128,
                10_000_000_000_000_000i128,
            ))
            .saturating_abs()
        })
        .map(|x| {
            x.cmp(&FixedI128::saturating_from_rational(
                1i128,
                10_000_000_000_000i128,
            ))
        });
    assert_eq!(delta, Some(Ordering::Less));
}

#[test]
fn get_d_empty() {
    let xp = vec![];
    let amp = FixedI128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_d(&xp, ann);

    assert_eq!(result, None);
}

#[test]
fn get_y_successful() {
    let i = 0;
    let j = 1;
    let x = FixedI128::saturating_from_rational(111, 100);
    let xp = vec![
        FixedI128::saturating_from_rational(11, 10),
        FixedI128::saturating_from_rational(88, 100),
    ];
    let amp = FixedI128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_y(i, j, x, &xp, ann);

    // expected y is 0.8703405416689252
    // expected precision is 1e-13
    let delta = result
        .map(|x| {
            x.saturating_sub(FixedI128::saturating_from_rational(
                8703405416689252i128,
                10_000_000_000_000_000i128,
            ))
            .saturating_abs()
        })
        .map(|x| {
            x.cmp(&FixedI128::saturating_from_rational(
                1,
                10_000_000_000_000i128,
            ))
        });
    assert_eq!(delta, Some(Ordering::Less));
}

#[test]
fn get_y_same_coin() {
    let i = 1;
    let j = 1;
    let x = FixedI128::saturating_from_rational(111, 100);
    let xp = vec![
        FixedI128::saturating_from_rational(11, 10),
        FixedI128::saturating_from_rational(88, 100),
    ];
    let amp = FixedI128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_y(i, j, x, &xp, ann);

    assert_eq!(result, None);
}

#[test]
fn get_y_i_greater_than_n() {
    let i = 33;
    let j = 1;
    let x = FixedI128::saturating_from_rational(111, 100);
    let xp = vec![
        FixedI128::saturating_from_rational(11, 10),
        FixedI128::saturating_from_rational(88, 100),
    ];
    let amp = FixedI128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_y(i, j, x, &xp, ann);

    assert_eq!(result, None);
}

#[test]
fn get_y_j_greater_than_n() {
    let i = 1;
    let j = 33;
    let x = FixedI128::saturating_from_rational(111, 100);
    let xp = vec![
        FixedI128::saturating_from_rational(11, 10),
        FixedI128::saturating_from_rational(88, 100),
    ];
    let amp = FixedI128::saturating_from_rational(292, 100);
    let ann = CurveAmm::get_ann(amp, xp.len()).unwrap();

    let result = CurveAmm::get_y(i, j, x, &xp, ann);

    assert_eq!(result, None);
}
