//! Runtime API definition for `equilibrium-curve-amm` pallet.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_runtime::traits::MaybeDisplay;

pub use equilibrium_curve_amm::{PoolId, PoolTokenIndex};

sp_api::decl_runtime_apis! {
    pub trait EquilibriumCurveAmmApi<Balance> where
        Balance: Codec + MaybeDisplay,
    {
        fn get_dy(pool_id: PoolId, i: PoolTokenIndex, j: PoolTokenIndex, dx: Balance) -> Option<Balance>;
        fn get_withdraw_one_coin(pool_id: PoolId, burn_amount: Balance, i: PoolTokenIndex) -> Option<Balance>;
        fn get_virtual_price(pool_id: PoolId) -> Option<Balance>;
    }
}
