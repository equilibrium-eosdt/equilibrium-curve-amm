use crate::{mock::*, Error};
use core::convert::From;
use frame_support::{assert_noop, assert_ok, traits::Currency};
use sp_runtime::FixedI128;
use sp_runtime::Permill;

#[test]
fn it_works_for_create_pool() {
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
