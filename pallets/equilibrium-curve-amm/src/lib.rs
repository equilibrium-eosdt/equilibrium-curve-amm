//! # Project description
//!
//! Substrate-based runtime version of Curve automated market maker will be a 7 week long project which will deliver a following functionality:
//!
//! - Low slippage, high efficiency stablecoin exchange
//!
//! - High efficiency exchange tool for other homogeneous assets on Polkadot (e.g. wrapped assets)
//!
//! - Low risk fee income for liquidity providers.
//!
//! - Liquidity superfluidity with additional rewards from supplying liquidity to lending protocols such as Equilibrium and Acala.
//!
//! Curve AMM is of paramount importance to the entire Polkadot ecosystem. With the introduction of parachains and interconnection of different Polka-based projects the issue of multiple wrapped assets representing the same underlying assets arises.
//!
//! Consider ETH, for example. There are multiple bridging solutions who promise to introduce wrapped-ETH and other ERC-20 tokens to Polkadot. There needs to be a way to manage or exchange all of these representations of the same underlying asset inside Polkadot with low cost and low slippage, and that is where the Curve AMM comes into play.
//!
//! Curve’s unique stableswap invariant utilizes liquidity much more efficiently compared to all existing DEXes for stablecoins at already several hundred USD TVL (total value locked). Since initial liquidity on Polkadot is hardly going to be very large, proposed efficiency is VERY important for the ecosystem to flourish.
//!
//! ## Deep Dive
//!
//! - See original [Curve Whitepaper](https://curve.fi/files/stableswap-paper.pdf) to read about StableSwap invariant.
//! - See original [Curve pool-template](https://github.com/curvefi/curve-contract/tree/master/contracts/pool-templates/base) source code this pallet is based on.
//! - See additional description about `get_y` function internals in [Deducing get_y formulas from StableSwap invariant](https://github.com/equilibrium-eosdt/equilibrium-curve-amm/blob/master/docs/deducing-get_y-formulas.pdf) paper.
//!
//! ## Integration Into a Chain
//!
//! See [Integration Guide](https://github.com/equilibrium-eosdt/equilibrium-curve-amm/blob/master/docs/INTEGRATION.md).
//!
//! ## Usage
//!
//! The `equilibrium-curve-amm` pallet provides its functionality to other pallets through [`CurveAmm`](traits::CurveAmm) trait implementation.

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate sp_runtime;

pub use pallet::*;

pub mod benchmarking;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
pub mod weights;

use crate::traits::CurveAmm;
use frame_support::codec::{Decode, Encode};
use frame_support::dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo};
use frame_support::ensure;
use frame_support::traits::{Currency, ExistenceRequirement, Get, OnUnbalanced, WithdrawReasons};
use sp_runtime::traits::{
    AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Convert,
};
use sp_runtime::Permill;
use sp_std::collections::btree_set::BTreeSet;
use sp_std::iter::FromIterator;
use sp_std::prelude::*;
use traits::{Assets, CheckedConvert, OnPoolCreated, SliceChecker};

pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use super::{traits::CheckedConvert, PoolId, PoolInfo, PoolTokenIndex};
    #[cfg(feature = "runtime-benchmarks")]
    use crate::traits::BenchmarkingInit;
    use crate::traits::{CurveAmm, SliceChecker};
    use crate::WeightInfo;
    use frame_support::{
        dispatch::{Codec, DispatchResultWithPostInfo},
        pallet_prelude::*,
        traits::{Currency, OnUnbalanced},
        PalletId,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Convert};
    use sp_runtime::Permill;
    use sp_std::prelude::*;

    /// Config of Equilibrium Curve Amm pallet
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The asset ID type
        type AssetId: Parameter + Ord + Copy;
        /// The balance type of an account
        type Balance: Parameter + Codec + Copy + Ord + CheckedAdd + CheckedSub;
        /// External implementation for required operations with assets
        type Assets: super::traits::Assets<Self::AssetId, Self::Balance, Self::AccountId>;
        /// Standard balances pallet for utility token or adapter
        type Currency: Currency<Self::AccountId, Balance = Self::Balance>;
        /// Anti ddos fee for pool creation
        #[pallet::constant]
        type CreationFee: Get<Self::Balance>;
        /// What to do with fee (burn, transfer to treasury, etc)
        type OnUnbalanced: OnUnbalanced<
            <Self::Currency as Currency<Self::AccountId>>::NegativeImbalance,
        >;
        /// Module account
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The number type for underlying calculations
        type Number: Parameter + CheckedAdd + CheckedSub + CheckedMul + CheckedDiv + Copy + Eq + Ord;
        /// Value that represents precision used for fixed-point iteration method
        type Precision: Get<Self::Number>;
        /// Conversions between `Self::Number` and various representations
        type Convert: Convert<Permill, Self::Number>
            + Convert<Self::Balance, Self::Number>
            + Convert<u8, Self::Number>
            + CheckedConvert<usize, Self::Number>
            + Convert<Self::Number, Self::Balance>;
        /// Provides a way to perform additional asset checks
        type AssetChecker: SliceChecker<Self::AssetId>;
        /// Event on new pool creation
        type OnPoolCreated: super::traits::OnPoolCreated;
        /// Weight information for extrinsics in this pallet
        type WeightInfo: WeightInfo;
        #[cfg(feature = "runtime-benchmarks")]
        type BenchmarkingInit: BenchmarkingInit;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Current number of pools (also ID for the next created pool)
    #[pallet::storage]
    #[pallet::getter(fn pool_count)]
    pub type PoolCount<T: Config> = StorageValue<_, PoolId, ValueQuery>;

    /// Existing pools
    #[pallet::storage]
    #[pallet::getter(fn pools)]
    pub type Pools<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        PoolId,
        PoolInfo<T::AccountId, T::AssetId, T::Number, T::Balance>,
    >;

    /// Event type for Equilibrium Curve AMM pallet
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Pool with specified id `PoolId` was created successfully by `T::AccountId`.
        ///
        /// Included values are:
        /// - account identifier `T::AccountId`
        /// - pool identifier `PoolId`
        ///
        /// \[who, pool_id\]
        CreatePool(T::AccountId, PoolId),
        /// Liquidity added into the pool `PoolId` by `T::AccountId`.
        ///
        /// Included values are:
        /// - account identifier `T::AccountId`
        /// - pool identifier `PoolId`
        /// - added token amounts `Vec<T::Balance>`
        /// - charged fees `Vec<T::Balance>`
        /// - actual invariant `T::Balance`
        /// - actual token supply `T::Balance`
        /// - minted amount `T::Balance`
        ///
        /// \[who, pool_id, token_amounts, fees, invariant, token_supply, mint_amount\]
        AddLiquidity(
            T::AccountId,
            PoolId,
            Vec<T::Balance>,
            Vec<T::Balance>,
            T::Balance,
            T::Balance,
            T::Balance,
        ),
        /// Token exchange happened.
        ///
        /// Included values are:
        /// - account identifier `T::AccountId`
        /// - pool identifier `PoolId`
        /// - index of sent token `PoolTokenIndex`
        /// - amount of sent token `T::Balance`
        /// - index of received token `PoolTokenIndex`
        /// - amount of received token `T::Balance`
        /// - charged fee `T::Balance`
        ///
        /// \[who, pool_id, sent_token_index, sent_amount, received_token_index, received_amount, fee\]
        TokenExchange(
            T::AccountId,
            PoolId,
            PoolTokenIndex,
            T::Balance,
            PoolTokenIndex,
            T::Balance,
            T::Balance,
        ),
        /// Liquidity removed from pool `PoolId` by `T::AccountId` in balanced way.
        ///
        /// Included values are:
        /// - account identifier `T::AccountId`
        /// - pool identifier `PoolId`
        /// - removed token amounts `Vec<T::Balance>`
        /// - charged fees `Vec<T::Balance>`
        /// - actual token supply `T::Balance`
        ///
        /// \[who, pool_id, token_amounts, fees, token_supply\]
        RemoveLiquidity(
            T::AccountId,
            PoolId,
            Vec<T::Balance>,
            Vec<T::Balance>,
            T::Balance,
        ),
        /// Liquidity removed from pool `PoolId` by `T::AccountId` in imbalanced way.
        ///
        /// Included values are:
        /// - account identifier `T::AccountId`
        /// - pool identifier `PoolId`
        /// - removed token amounts `Vec<T::Balance>`
        /// - charged fees `Vec<T::Balance>`
        /// - actual invariant `T::Balance`
        /// - actual token supply `T::Balance`
        /// - removed LP token amount `T::Balance`
        ///
        /// \[who, pool_id, token_amounts, fees, invariant, token_supply, burn_amount\]
        RemoveLiquidityImbalance(
            T::AccountId,
            PoolId,
            Vec<T::Balance>,
            Vec<T::Balance>,
            T::Balance,
            T::Balance,
            T::Balance,
        ),
        /// Liquidity removed from pool `PoolId` only for one token.
        ///
        /// Included values are:
        /// - account identifier `T::AccountId`
        /// - pool identifier `PoolId`
        /// - removed token amount `T::Balance`
        /// - received token index `PoolTokenIndex`
        /// - received token amount `T::Balance`
        /// - actual token supply `T::Balance`
        /// - charged fee `T::Balance`
        ///
        /// \[who, pool_id, burn_amount, received_token, received_amount, token_supply, fee\]
        RemoveLiquidityOne(
            T::AccountId,
            PoolId,
            T::Balance,
            PoolTokenIndex,
            T::Balance,
            T::Balance,
            T::Balance,
        ),
        /// Withdraw admin fees `Vec<T::Balance>` from pool `PoolId` by user `T::AccountId`
        ///
        /// Included values are:
        /// - account identifier `T::AccountId`
        /// - pool identifier `PoolId`
        /// - withdrew admin fees `Vec<T::Balance>`
        ///
        /// [who, pool_id, admin_fees]
        WithdrawAdminFees(T::AccountId, PoolId, Vec<T::Balance>),
    }

    /// Error type for Equilibrium Curve AMM pallet
    #[pallet::error]
    pub enum Error<T> {
        /// Could not create new asset
        AssetNotCreated,
        /// Values in the storage are inconsistent
        InconsistentStorage,
        /// Not enough assets provided
        NotEnoughAssets,
        /// Some provided assets are not unique
        DuplicateAssets,
        /// Pool with specified id is not found
        PoolNotFound,
        /// Error occurred while performing math calculations
        Math,
        /// Specified asset amount is wrong
        WrongAssetAmount,
        /// Required amount of some token did not reached during adding or removing liquidity
        RequiredAmountNotReached,
        /// Source does not have required amount of coins to complete operation
        InsufficientFunds,
        /// Specified index is out of range
        IndexOutOfRange,
        /// The `AssetChecker` can use this error in case it can't provide better error
        ExternalAssetCheckFailed,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Creates a pool, taking a creation fee from the caller
        #[pallet::weight(T::WeightInfo::create_pool(assets.len() as u32))]
        pub fn create_pool(
            origin: OriginFor<T>,
            assets: Vec<T::AssetId>,
            amplification: T::Number,
            fee: Permill,
            admin_fee: Permill,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as CurveAmm>::create_pool(&who, assets, amplification, fee, admin_fee)
        }

        /// Deposit coins into the pool
        /// `amounts` - list of amounts of coins to deposit,
        /// `min_mint_amount` - minimum amount of LP tokens to mint from the deposit.
        #[pallet::weight(T::WeightInfo::add_liquidity(amounts.len() as u32))]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            pool_id: PoolId,
            amounts: Vec<T::Balance>,
            min_mint_amount: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as CurveAmm>::add_liquidity(&who, pool_id, amounts, min_mint_amount)
        }

        /// Perform an exchange between two coins.
        /// `i` - index value of the coin to send,
        /// `j` - index value of the coin to receive,
        /// `dx` - amount of `i` being exchanged,
        /// `min_dy` - minimum amount of `j` to receive.
        #[pallet::weight(T::WeightInfo::exchange())]
        pub fn exchange(
            origin: OriginFor<T>,
            pool_id: PoolId,
            i: PoolTokenIndex,
            j: PoolTokenIndex,
            dx: T::Balance,
            min_dy: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as CurveAmm>::exchange(&who, pool_id, i, j, dx, min_dy)
        }

        /// Withdraw coins from the pool.
        /// Withdrawal amount are based on current deposit ratios.
        /// `amount` - quantity of LP tokens to burn in the withdrawal,
        /// `min_amounts` - minimum amounts of underlying coins to receive.
        #[pallet::weight(T::WeightInfo::remove_liquidity(min_amounts.len() as u32))]
        pub fn remove_liquidity(
            origin: OriginFor<T>,
            pool_id: PoolId,
            amount: T::Balance,
            min_amounts: Vec<T::Balance>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as CurveAmm>::remove_liquidity(&who, pool_id, amount, min_amounts)
        }

        /// Withdraw coins from the pool in an imbalanced amount.
        /// `amounts` - list of amounts of underlying coins to withdraw,
        /// `max_burn_amount` - maximum amount of LP token to burn in the withdrawal.
        #[pallet::weight(T::WeightInfo::remove_liquidity_imbalance(amounts.len() as u32))]
        pub fn remove_liquidity_imbalance(
            origin: OriginFor<T>,
            pool_id: PoolId,
            amounts: Vec<T::Balance>,
            max_burn_amount: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as CurveAmm>::remove_liquidity_imbalance(&who, pool_id, amounts, max_burn_amount)
        }

        /// Withdraw a single coin from the pool.
        /// `token_amount` - amount of LP tokens to burn in the withdrawal,
        /// `i` - index value of the coin to withdraw,
        /// `min_amount` - minimum amount of coin to receive.
        #[pallet::weight(T::WeightInfo::remove_liquidity_one_coin())]
        pub fn remove_liquidity_one_coin(
            origin: OriginFor<T>,
            pool_id: PoolId,
            token_amount: T::Balance,
            i: PoolTokenIndex,
            min_amount: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as CurveAmm>::remove_liquidity_one_coin(
                &who,
                pool_id,
                token_amount,
                i,
                min_amount,
            )
        }

        /// Withdraw admin fee.
        #[pallet::weight(T::WeightInfo::withdraw_admin_fees())]
        pub fn withdraw_admin_fees(
            origin: OriginFor<T>,
            pool_id: PoolId,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            <Self as CurveAmm>::withdraw_admin_fees(&who, pool_id)
        }
    }
}

// The main implementation block for the module.
impl<T: Config> Pallet<T> {
    pub(crate) fn get_number(n: u8) -> T::Number {
        <T::Convert as Convert<u8, T::Number>>::convert(n)
    }

    pub(crate) fn convert_number_to_balance(number: T::Number) -> T::Balance {
        <T::Convert as Convert<T::Number, T::Balance>>::convert(number)
    }

    pub(crate) fn convert_balance_to_number(balance: T::Balance) -> T::Number {
        <T::Convert as Convert<T::Balance, T::Number>>::convert(balance)
    }

    pub(crate) fn convert_vec_number_to_balance(numbers: Vec<T::Number>) -> Vec<T::Balance> {
        numbers
            .into_iter()
            .map(Self::convert_number_to_balance)
            .collect()
    }

    pub(crate) fn convert_vec_balance_to_number(balances: Vec<T::Balance>) -> Vec<T::Number> {
        balances
            .into_iter()
            .map(Self::convert_balance_to_number)
            .collect()
    }

    /// Find `ann = amp * n^n` where `amp` - amplification coefficient,
    /// `n` - number of coins.
    pub(crate) fn get_ann(amp: T::Number, n: usize) -> Option<T::Number> {
        let n_coins = <T::Convert as CheckedConvert<usize, T::Number>>::convert(n)?;
        let mut ann = amp;
        for _ in 0..n {
            ann = ann.checked_mul(&n_coins)?;
        }
        Some(ann)
    }

    /// Find `d` preserving StableSwap invariant.
    /// Here `d` - total amount of coins when they have an equal price,
    /// `xp` - coin amounts, `ann` is amplification coefficient multiplied by `n^n`,
    /// where `n` is number of coins.
    ///
    /// # Notes
    ///
    /// D invariant calculation in non-overflowing integer operations iteratively
    ///
    /// ```pseudocode
    ///  A * sum(x_i) * n^n + D = A * D * n^n + D^(n+1) / (n^n * prod(x_i))
    /// ```
    ///
    /// Converging solution:
    ///
    /// ```pseudocode
    /// D[j + 1] = (A * n^n * sum(x_i) - D[j]^(n+1) / (n^n * prod(x_i))) / (A * n^n - 1)
    /// ```
    pub(crate) fn get_d(xp: &[T::Number], ann: T::Number) -> Option<T::Number> {
        let prec = T::Precision::get();
        let zero = Self::get_number(0);
        let one = Self::get_number(1);

        let n_coins = <T::Convert as CheckedConvert<usize, T::Number>>::convert(xp.len())?;

        let mut s = zero;

        for x in xp.iter() {
            s = s.checked_add(x)?;
        }
        if s == zero {
            return Some(zero);
        }

        let mut d = s;

        for _ in 0..255 {
            let mut d_p = d;
            for x in xp.iter() {
                // d_p = d_p * d / (x * n_coins)
                d_p = d_p
                    .checked_mul(&d)?
                    .checked_div(&x.checked_mul(&n_coins)?)?;
            }
            let d_prev = d;
            // d = (ann * s + d_p * n_coins) * d / ((ann - 1) * d + (n_coins + 1) * d_p)
            d = ann
                .checked_mul(&s)?
                .checked_add(&d_p.checked_mul(&n_coins)?)?
                .checked_mul(&d)?
                .checked_div(
                    &ann.checked_sub(&one)?
                        .checked_mul(&d)?
                        .checked_add(&n_coins.checked_add(&one)?.checked_mul(&d_p)?)?,
                )?;

            if d > d_prev {
                if d.checked_sub(&d_prev)? <= prec {
                    return Some(d);
                }
            } else {
                if d_prev.checked_sub(&d)? <= prec {
                    return Some(d);
                }
            }
        }
        // Convergence typically occurs in 4 rounds or less, this should be unreachable!
        None
    }

    /// Find new amount `xp[j]` if one changes some other amount `x[i]` to value `x` preserving StableSwap invariant.
    /// Here `xp` - coin amounts, `ann` is amplification coefficient multiplied by `n^n`, where
    /// `n` is number of coins.
    ///
    /// See https://github.com/equilibrium-eosdt/equilibrium-curve-amm/blob/master/docs/deducing-get_y-formulas.pdf
    /// for detailed explanation about formulas this function uses.
    ///
    /// # Notes
    ///
    /// Done by solving quadratic equation iteratively.
    ///
    /// ```pseudocode
    /// x_1^2 + x_1 * (sum' - (A * n^n - 1) * D / (A * n^n)) = D^(n+1) / (n^2n * prod' * A)
    /// x_1^2 + b * x_1 = c
    ///
    /// x_1 = (x_1^2 + c) / (2 * x_1 + b)
    /// ```
    pub(crate) fn get_y(
        i: usize,
        j: usize,
        x: T::Number,
        xp: &[T::Number],
        ann: T::Number,
    ) -> Option<T::Number> {
        let prec = T::Precision::get();
        let zero = Self::get_number(0);

        let two = Self::get_number(2);

        let n = <T::Convert as CheckedConvert<usize, T::Number>>::convert(xp.len())?;

        // Same coin
        if !(i != j) {
            return None;
        }
        // j above n
        if !(j < xp.len()) {
            return None;
        }
        if !(i < xp.len()) {
            return None;
        }

        let d = Self::get_d(xp, ann)?;

        let mut c = d;
        let mut s = zero;

        // Calculate s and c
        // p is implicitly calculated as part of c
        // note that loop makes n - 1 iterations
        for k in 0..xp.len() {
            let x_k;
            if k == i {
                x_k = x;
            } else if k != j {
                x_k = xp[k];
            } else {
                continue;
            }
            // s = s + x_k
            s = s.checked_add(&x_k)?;
            // c = c * d / (x_k * n)
            c = c.checked_mul(&d)?.checked_div(&x_k.checked_mul(&n)?)?;
        }
        // c = c * d / (ann * n)
        // At this step we have d^n in the numerator of c
        // and n^(n-1) in its denominator.
        // So we multiplying it by remaining d/n
        c = c.checked_mul(&d)?.checked_div(&ann.checked_mul(&n)?)?;

        // b = s + d / ann
        // We subtract d later
        let b = s.checked_add(&d.checked_div(&ann)?)?;
        let mut y = d;

        for _ in 0..255 {
            let y_prev = y;
            // y = (y^2 + c) / (2 * y + b - d)
            // Subtract d to calculate b finally
            y = y
                .checked_mul(&y)?
                .checked_add(&c)?
                .checked_div(&two.checked_mul(&y)?.checked_add(&b)?.checked_sub(&d)?)?;

            // Equality with the specified precision
            if y > y_prev {
                if y.checked_sub(&y_prev)? <= prec {
                    return Some(y);
                }
            } else {
                if y_prev.checked_sub(&y)? <= prec {
                    return Some(y);
                }
            }
        }

        None
    }

    /// Calculate `x[i]` if one reduces `d` from being calculated for `xp` to `d`.
    ///
    /// # Notes
    ///
    /// Done by solving quadratic equation iteratively.
    ///
    /// ```pseudocode
    /// x_1^2 + x_1 * (sum' - (A * n^n - 1) * D / (A * n^n)) = D^(n+1) / (n^2n * prod' * A)
    /// x_1^2 + b * x_1 = c
    ///
    /// x_1 = (x_1^2 + c) / (2 * x_1 + b)
    /// ```
    pub(crate) fn get_y_d(
        i: usize,
        d: T::Number,
        xp: &[T::Number],
        ann: T::Number,
    ) -> Option<T::Number> {
        let prec = T::Precision::get();
        let zero = Self::get_number(0);
        let two = Self::get_number(2);

        let n_coins = <T::Convert as CheckedConvert<usize, T::Number>>::convert(xp.len())?;

        if i >= xp.len() {
            return None;
        }

        let mut c = d;
        let mut s = zero;

        for k in 0..xp.len() {
            if k == i {
                continue;
            }

            let x = xp[k];

            s = s.checked_add(&x)?;
            // c = c * d / (x * n_coins)
            c = c.checked_mul(&d)?.checked_div(&x.checked_mul(&n_coins)?)?;
        }
        // c = c * d / (ann * n_coins)
        c = c
            .checked_mul(&d)?
            .checked_div(&ann.checked_mul(&n_coins)?)?;
        // b = s + d / ann
        let b = s.checked_add(&d.checked_div(&ann)?)?;
        let mut y = d;

        for _ in 0..255 {
            let y_prev = y;
            // y = (y*y + c) / (2 * y + b - d)
            y = y
                .checked_mul(&y)?
                .checked_add(&c)?
                .checked_div(&two.checked_mul(&y)?.checked_add(&b)?.checked_sub(&d)?)?;

            // Equality with the specified precision
            if y > y_prev {
                if y.checked_sub(&y_prev)? <= prec {
                    return Some(y);
                }
            } else {
                if y_prev.checked_sub(&y)? <= prec {
                    return Some(y);
                }
            }
        }

        None
    }

    /// First, need to calculate:
    /// - current `d`,
    /// - solve equation against `y_i` for `d` - token_amount.
    pub(crate) fn calc_withdraw_one_coin(
        token_amount: T::Number,
        i: usize,
        xp: &[T::Number],
        ann: T::Number,
        total_supply: T::Number,
        pool_fee: T::Number,
    ) -> Option<(T::Number, T::Number)> {
        let prec = T::Precision::get();
        let one = Self::get_number(1);
        let four = Self::get_number(4);

        let n_coins = <T::Convert as CheckedConvert<usize, T::Number>>::convert(xp.len())?;

        let d0 = Self::get_d(xp, ann)?;
        // d1 = d0 - token_amount * d0 / total_supply
        let d1 = d0.checked_sub(&token_amount.checked_mul(&d0)?.checked_div(&total_supply)?)?;
        let new_y = Self::get_y_d(i, d1, xp, ann)?;
        let mut xp_reduced = xp.to_vec();

        // Deposit x + withdraw y would charge about same
        // fees as a swap. Otherwise, one could exchange w/o paying fees.
        // And this formula leads to exactly that equality
        // fee = pool_fee * n_coins / (4 * (n_coins - 1))
        let fee = pool_fee
            .checked_mul(&n_coins)?
            .checked_div(&four.checked_mul(&n_coins.checked_sub(&one)?)?)?;

        for j in 0..xp.len() {
            let dx_expected = if j == i {
                // dx_expected = xp[j] * d1 / d0 - new_y
                xp[j]
                    .checked_mul(&d1)?
                    .checked_div(&d0)?
                    .checked_sub(&new_y)?
            } else {
                // dx_expected = xp[j] - xp[j] * d1 / d0
                xp[j].checked_sub(&xp[j].checked_mul(&d1)?.checked_div(&d0)?)?
            };
            // xp_reduced[j] = xp_reduced[j] - fee * dx_expected
            xp_reduced[j] = xp_reduced[j].checked_sub(&fee.checked_mul(&dx_expected)?)?;
        }

        let dy = xp_reduced[i].checked_sub(&Self::get_y_d(i, d1, &xp_reduced, ann)?)?;
        // Withdraw less to account for rounding errors
        let dy = dy.checked_sub(&prec)?;
        // Without fees
        let dy_0 = xp[i].checked_sub(&new_y)?;

        let fee = dy_0.checked_sub(&dy)?;

        Some((dy, fee))
    }

    /// Calculates the amount received when withdrawing a single coin.
    pub fn get_withdraw_one_coin(
        pool_id: PoolId,
        token_amount: T::Balance,
        i: PoolTokenIndex,
    ) -> Result<T::Balance, DispatchError> {
        let n_token_amount = Self::convert_balance_to_number(token_amount);
        let i = i as usize;
        let pool = Self::pools(pool_id).ok_or(Error::<T>::PoolNotFound)?;
        let pool_balances = Self::convert_vec_balance_to_number(pool.balances.clone());
        let n_coins = pool.assets.len();
        let ann = Self::get_ann(pool.amplification, n_coins).ok_or(Error::<T>::Math)?;
        let token_supply =
            Self::convert_balance_to_number(T::Assets::total_issuance(pool.pool_asset));
        let pool_fee = <T::Convert as Convert<Permill, T::Number>>::convert(pool.fee);

        let (dy, _) = Self::calc_withdraw_one_coin(
            n_token_amount,
            i,
            &pool_balances,
            ann,
            token_supply,
            pool_fee,
        )
        .ok_or(Error::<T>::Math)?;

        Ok(Self::convert_number_to_balance(dy))
    }

    /// Calculates exchange outcome `dy` for a given `i`, `j` and `dx` values.
    pub fn get_dy(
        pool_id: PoolId,
        i: PoolTokenIndex,
        j: PoolTokenIndex,
        dx: T::Balance,
    ) -> Result<T::Balance, DispatchError> {
        let i = i as usize;
        let j = j as usize;

        let pool = Self::pools(pool_id).ok_or(Error::<T>::PoolNotFound)?;

        let xp = Self::convert_vec_balance_to_number(pool.balances);

        ensure!(i < xp.len() && j < xp.len(), Error::<T>::IndexOutOfRange);

        let x = xp[i]
            .checked_add(&Self::convert_balance_to_number(dx))
            .ok_or(Error::<T>::Math)?;

        let n_coins = pool.assets.len();
        let ann = Self::get_ann(pool.amplification, n_coins).ok_or(Error::<T>::Math)?;
        let y = Self::get_y(i, j, x, &xp, ann).ok_or(Error::<T>::Math)?;

        let dy = (|| {
            let prec = T::Precision::get();

            xp[j].checked_sub(&y)?.checked_sub(&prec)
        })()
        .ok_or(Error::<T>::Math)?;

        let fee = <T::Convert as Convert<Permill, T::Number>>::convert(pool.fee);

        let dy = Self::convert_number_to_balance(dy.checked_sub(&fee).ok_or(Error::<T>::Math)?);

        Ok(dy)
    }

    /// The current virtual price of the pool LP token.
    pub fn get_virtual_price(pool_id: PoolId) -> Result<T::Balance, DispatchError> {
        let pool = Self::pools(pool_id).ok_or(Error::<T>::PoolNotFound)?;

        let xp = Self::convert_vec_balance_to_number(pool.balances);

        let n_coins = pool.assets.len();
        let ann = Self::get_ann(pool.amplification, n_coins).ok_or(Error::<T>::Math)?;

        let d = Self::get_d(&xp, ann).ok_or(Error::<T>::Math)?;

        let token_supply = T::Assets::total_issuance(pool.pool_asset);

        let ratio = d
            .checked_div(&Self::convert_balance_to_number(token_supply))
            .ok_or(Error::<T>::Math)?;

        Ok(Self::convert_number_to_balance(ratio))
    }

    fn transfer_liquidity_into_pool(
        pool: &mut PoolInfo<T::AccountId, T::AssetId, T::Number, T::Balance>,
        source: &T::AccountId,
        destination_asset_index: usize,
        amount: T::Balance,
    ) -> DispatchResult {
        T::Assets::transfer(
            pool.assets[destination_asset_index],
            source,
            &T::PalletId::get().into_account(),
            amount,
        )?;

        pool.total_balances[destination_asset_index] = pool.total_balances[destination_asset_index]
            .checked_add(&amount)
            .ok_or(Error::<T>::InconsistentStorage)?;

        Ok(())
    }

    fn transfer_liquidity_from_pool(
        pool: &mut PoolInfo<T::AccountId, T::AssetId, T::Number, T::Balance>,
        source_asset_index: usize,
        destination: &T::AccountId,
        amount: T::Balance,
    ) -> DispatchResult {
        T::Assets::transfer(
            pool.assets[source_asset_index],
            &T::PalletId::get().into_account(),
            destination,
            amount,
        )?;

        pool.total_balances[source_asset_index] = pool.total_balances[source_asset_index]
            .checked_sub(&amount)
            .ok_or(Error::<T>::InconsistentStorage)?;

        Ok(())
    }
}

impl<T: Config> CurveAmm for Pallet<T> {
    type AssetId = T::AssetId;
    type Number = T::Number;
    type Balance = T::Balance;
    type AccountId = T::AccountId;

    fn pool_count() -> PoolId {
        PoolCount::<T>::get()
    }

    fn pool(
        id: PoolId,
    ) -> Option<PoolInfo<Self::AccountId, Self::AssetId, Self::Number, Self::Balance>> {
        Pools::<T>::get(id)
    }

    fn create_pool(
        who: &Self::AccountId,
        assets: Vec<Self::AssetId>,
        amplification: Self::Number,
        fee: Permill,
        admin_fee: Permill,
    ) -> DispatchResultWithPostInfo {
        // Assets related checks
        ensure!(assets.len() > 1, Error::<T>::NotEnoughAssets);
        let unique_assets = BTreeSet::<T::AssetId>::from_iter(assets.iter().copied());
        ensure!(
            unique_assets.len() == assets.len(),
            Error::<T>::DuplicateAssets
        );

        let _ = T::AssetChecker::check(&assets)?;

        // Take fee
        let creation_fee = T::CreationFee::get();
        let imbalance = T::Currency::withdraw(
            &who,
            creation_fee,
            WithdrawReasons::FEE,
            ExistenceRequirement::AllowDeath,
        )
        .map_err(|_| Error::<T>::InsufficientFunds)?;
        T::OnUnbalanced::on_unbalanced(imbalance);

        // Add new pool
        let pool_id = PoolCount::<T>::try_mutate(|pool_count| -> Result<PoolId, DispatchError> {
            let pool_id = *pool_count;

            Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
                // We expect that PoolInfos have sequential keys.
                // No PoolInfo can have key greater or equal to PoolCount
                ensure!(maybe_pool_info.is_none(), Error::<T>::InconsistentStorage);

                let asset =
                    T::Assets::create_asset(pool_id).map_err(|_| Error::<T>::AssetNotCreated)?;

                let empty_balances =
                    vec![Self::convert_number_to_balance(Self::get_number(0)); assets.len()];

                *maybe_pool_info = Some(PoolInfo {
                    owner: who.clone(),
                    pool_asset: asset,
                    assets,
                    amplification,
                    fee,
                    admin_fee,
                    balances: empty_balances.clone(),
                    total_balances: empty_balances,
                });

                Ok(())
            })?;

            *pool_count = pool_id
                .checked_add(1)
                .ok_or(Error::<T>::InconsistentStorage)?;

            Ok(pool_id)
        })?;

        T::OnPoolCreated::on_pool_created(pool_id);

        Self::deposit_event(Event::CreatePool(who.clone(), pool_id));

        Ok(().into())
    }

    fn add_liquidity(
        who: &Self::AccountId,
        pool_id: PoolId,
        amounts: Vec<Self::Balance>,
        min_mint_amount: Self::Balance,
    ) -> DispatchResultWithPostInfo {
        let zero = Self::get_number(0);

        let b_zero = Self::convert_number_to_balance(zero);
        ensure!(
            amounts.iter().all(|&x| x >= b_zero),
            Error::<T>::WrongAssetAmount
        );

        let (provider, pool_id, token_amounts, fees, invariant, token_supply, mint_amount) =
            Pools::<T>::try_mutate(pool_id, |pool| -> Result<_, DispatchError> {
                let pool = pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;

                let n_coins = pool.assets.len();

                ensure!(
                    n_coins == pool.balances.len(),
                    Error::<T>::InconsistentStorage
                );

                ensure!(n_coins == amounts.len(), Error::<T>::IndexOutOfRange);

                let ann = Self::get_ann(pool.amplification, n_coins).ok_or(Error::<T>::Math)?;

                let old_balances = Self::convert_vec_balance_to_number(pool.balances.clone());

                let d0 = Self::get_d(&old_balances, ann).ok_or(Error::<T>::Math)?;

                let token_supply =
                    Self::convert_balance_to_number(T::Assets::total_issuance(pool.pool_asset));
                let mut new_balances = old_balances.clone();
                let n_amounts = amounts
                    .iter()
                    .copied()
                    .map(Self::convert_balance_to_number)
                    .collect::<Vec<_>>();
                for i in 0..n_coins {
                    if token_supply == zero {
                        ensure!(n_amounts[i] > zero, Error::<T>::WrongAssetAmount);
                    }
                    new_balances[i] = new_balances[i]
                        .checked_add(&n_amounts[i])
                        .ok_or(Error::<T>::Math)?;
                }

                let d1 = Self::get_d(&new_balances, ann).ok_or(Error::<T>::Math)?;
                ensure!(d1 > d0, Error::<T>::WrongAssetAmount);

                let mut fees = vec![zero; n_coins];
                let mint_amount;

                // Only account for fees if we are not the first to deposit
                if token_supply > zero {
                    // Deposit x + withdraw y would charge about same
                    // fees as a swap. Otherwise, one could exchange w/o paying fees.
                    // And this formula leads to exactly that equality
                    // fee = pool.fee * n_coins / (4 * (n_coins - 1))
                    let fee = (|| {
                        let n_coins =
                            <T::Convert as CheckedConvert<usize, T::Number>>::convert(n_coins)?;
                        let one = Self::get_number(1);
                        let four = Self::get_number(4);

                        <T::Convert as Convert<Permill, T::Number>>::convert(pool.fee)
                            .checked_mul(&n_coins)?
                            .checked_div(&four.checked_mul(&n_coins.checked_sub(&one)?)?)
                    })()
                    .ok_or(Error::<T>::Math)?;
                    let admin_fee =
                        <T::Convert as Convert<Permill, T::Number>>::convert(pool.admin_fee);
                    for i in 0..n_coins {
                        // ideal_balance = d1 * old_balances[i] / d0
                        let ideal_balance =
                            (|| d1.checked_mul(&old_balances[i])?.checked_div(&d0))()
                                .ok_or(Error::<T>::Math)?;

                        let new_balance = new_balances[i];

                        // difference = abs(ideal_balance - new_balance)
                        let difference = (if ideal_balance > new_balance {
                            ideal_balance.checked_sub(&new_balance)
                        } else {
                            new_balance.checked_sub(&ideal_balance)
                        })
                        .ok_or(Error::<T>::Math)?;

                        fees[i] = fee.checked_mul(&difference).ok_or(Error::<T>::Math)?;
                        // new_pool_balance = new_balance - (fees[i] * admin_fee)
                        let new_pool_balance =
                            (|| new_balance.checked_sub(&fees[i].checked_mul(&admin_fee)?))()
                                .ok_or(Error::<T>::Math)?;
                        pool.balances[i] = Self::convert_number_to_balance(new_pool_balance);

                        new_balances[i] = new_balances[i]
                            .checked_sub(&fees[i])
                            .ok_or(Error::<T>::Math)?;
                    }
                    let d2 = Self::get_d(&new_balances, ann).ok_or(Error::<T>::Math)?;

                    // mint_amount = token_supply * (d2 - d0) / d0
                    mint_amount = (|| {
                        token_supply
                            .checked_mul(&d2.checked_sub(&d0)?)?
                            .checked_div(&d0)
                    })()
                    .ok_or(Error::<T>::Math)?;
                } else {
                    pool.balances = Self::convert_vec_number_to_balance(new_balances);
                    mint_amount = d1;
                }

                ensure!(
                    mint_amount >= Self::convert_balance_to_number(min_mint_amount),
                    Error::<T>::RequiredAmountNotReached
                );

                let new_token_supply = token_supply
                    .checked_add(&mint_amount)
                    .ok_or(Error::<T>::Math)?;

                // Ensure that for all tokens user has sufficient amount
                for i in 0..n_coins {
                    ensure!(
                        T::Assets::balance(pool.assets[i], &who) >= amounts[i],
                        Error::<T>::InsufficientFunds
                    );
                }
                for i in 0..n_coins {
                    if n_amounts[i] > zero {
                        Self::transfer_liquidity_into_pool(pool, &who, i, amounts[i])?;
                    }
                }

                T::Assets::mint(
                    pool.pool_asset,
                    &who,
                    Self::convert_number_to_balance(mint_amount),
                )?;

                let fees = fees
                    .into_iter()
                    .map(|x| Self::convert_number_to_balance(x))
                    .collect::<Vec<T::Balance>>();

                Ok((
                    who.clone(),
                    pool_id,
                    amounts,
                    fees,
                    Self::convert_number_to_balance(d1),
                    Self::convert_number_to_balance(new_token_supply),
                    Self::convert_number_to_balance(mint_amount),
                ))
            })?;

        Self::deposit_event(Event::AddLiquidity(
            provider,
            pool_id,
            token_amounts,
            fees,
            invariant,
            token_supply,
            mint_amount,
        ));

        Ok(().into())
    }

    fn exchange(
        who: &Self::AccountId,
        pool_id: PoolId,
        i: PoolTokenIndex,
        j: PoolTokenIndex,
        dx: Self::Balance,
        min_dy: Self::Balance,
    ) -> DispatchResultWithPostInfo {
        let prec = T::Precision::get();

        let b_zero = Self::convert_number_to_balance(Self::get_number(0));
        ensure!(dx >= b_zero, Error::<T>::WrongAssetAmount);

        // sold_id, tokens_sold, bought_id, tokens_bought
        let (provider, pool_id, dy, fee) =
            Pools::<T>::try_mutate(pool_id, |pool| -> Result<_, DispatchError> {
                let pool = pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;

                let i = i as usize;
                let j = j as usize;

                let n_coins = pool.assets.len();

                ensure!(i < n_coins && j < n_coins, Error::<T>::IndexOutOfRange);

                let n_dx = Self::convert_balance_to_number(dx);
                let n_min_dy = Self::convert_balance_to_number(min_dy);

                let xp = Self::convert_vec_balance_to_number(pool.balances.clone());

                // xp[i] + dx
                let x = xp[i].checked_add(&n_dx).ok_or(Error::<T>::Math)?;

                let ann = Self::get_ann(pool.amplification, n_coins).ok_or(Error::<T>::Math)?;
                let y = Self::get_y(i, j, x, &xp, ann).ok_or(Error::<T>::Math)?;

                // -1 just in case there were some rounding errors
                // dy = xp[j] - y - 1
                let n_dy =
                    (|| xp[j].checked_sub(&y)?.checked_sub(&prec))().ok_or(Error::<T>::Math)?;

                let fee = <T::Convert as Convert<Permill, T::Number>>::convert(pool.fee);
                let dy_fee = n_dy.checked_mul(&fee).ok_or(Error::<T>::Math)?;
                let n_dy = n_dy.checked_sub(&dy_fee).ok_or(Error::<T>::Math)?;
                ensure!(n_dy >= n_min_dy, Error::<T>::RequiredAmountNotReached);

                let admin_fee =
                    <T::Convert as Convert<Permill, T::Number>>::convert(pool.admin_fee);
                let dy_admin_fee = dy_fee.checked_mul(&admin_fee).ok_or(Error::<T>::Math)?;

                pool.balances[i] = Self::convert_number_to_balance(
                    xp[i].checked_add(&n_dx).ok_or(Error::<T>::Math)?,
                );
                // When rounding errors happen, we undercharge admin fee in favor of LP
                // pool.balances[j] = xp[j] - n_dy - dy_admin_fee
                pool.balances[j] = Self::convert_number_to_balance(
                    (|| xp[j].checked_sub(&n_dy)?.checked_sub(&dy_admin_fee))()
                        .ok_or(Error::<T>::Math)?,
                );

                let dy = Self::convert_number_to_balance(n_dy);

                ensure!(
                    T::Assets::balance(pool.assets[i], &who) >= dx,
                    Error::<T>::InsufficientFunds
                );

                ensure!(
                    T::Assets::balance(pool.assets[j], &T::PalletId::get().into_account()) >= dy,
                    Error::<T>::InsufficientFunds
                );

                Self::transfer_liquidity_into_pool(pool, &who, i, dx)?;
                Self::transfer_liquidity_from_pool(pool, j, &who, dy)?;

                Ok((
                    who.clone(),
                    pool_id,
                    dy,
                    Self::convert_number_to_balance(dy_fee),
                ))
            })?;

        Self::deposit_event(Event::TokenExchange(provider, pool_id, i, dx, j, dy, fee));

        Ok(().into())
    }

    fn remove_liquidity(
        who: &Self::AccountId,
        pool_id: PoolId,
        amount: Self::Balance,
        min_amounts: Vec<Self::Balance>,
    ) -> DispatchResultWithPostInfo {
        let zero = Self::get_number(0);

        let b_zero = Self::convert_number_to_balance(zero);
        ensure!(amount >= b_zero, Error::<T>::WrongAssetAmount);

        let n_amount = Self::convert_balance_to_number(amount);

        let min_amounts = min_amounts
            .into_iter()
            .map(Self::convert_balance_to_number)
            .collect::<Vec<_>>();

        let (provider, pool_id, token_amounts, fees, token_supply) =
            Pools::<T>::try_mutate(pool_id, |pool| -> Result<_, DispatchError> {
                let pool = pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;

                let n_coins = pool.assets.len();

                ensure!(
                    n_coins == pool.balances.len(),
                    Error::<T>::InconsistentStorage
                );

                ensure!(n_coins == min_amounts.len(), Error::<T>::IndexOutOfRange);

                let token_supply =
                    Self::convert_balance_to_number(T::Assets::total_issuance(pool.pool_asset));

                let mut n_amounts = vec![zero; n_coins];

                for i in 0..n_coins {
                    let old_balance = Self::convert_balance_to_number(pool.balances[i]);
                    // value = old_balance * n_amount / token_supply
                    let value = (|| {
                        old_balance
                            .checked_mul(&n_amount)?
                            .checked_div(&token_supply)
                    })()
                    .ok_or(Error::<T>::Math)?;
                    ensure!(
                        value >= min_amounts[i],
                        Error::<T>::RequiredAmountNotReached
                    );

                    // pool.balances[i] = old_balance - value
                    pool.balances[i] = Self::convert_number_to_balance(
                        old_balance
                            .checked_sub(&value)
                            .ok_or(Error::<T>::InsufficientFunds)?,
                    );

                    n_amounts[i] = value;
                }

                let amounts = n_amounts
                    .iter()
                    .copied()
                    .map(Self::convert_number_to_balance)
                    .collect::<Vec<T::Balance>>();

                let new_token_supply = token_supply
                    .checked_sub(&n_amount)
                    .ok_or(Error::<T>::Math)?;

                let fees = vec![Self::convert_number_to_balance(zero); n_coins];

                T::Assets::burn(pool.pool_asset, &who, amount)?;

                // Ensure that for all tokens we have sufficient amount
                for i in 0..n_coins {
                    ensure!(
                        T::Assets::balance(pool.assets[i], &T::PalletId::get().into_account())
                            >= amounts[i],
                        Error::<T>::InsufficientFunds
                    );
                }

                for i in 0..n_coins {
                    if n_amounts[i] > zero {
                        Self::transfer_liquidity_from_pool(pool, i, &who, amounts[i])?;
                    }
                }

                Ok((
                    who.clone(),
                    pool_id,
                    amounts,
                    fees,
                    Self::convert_number_to_balance(new_token_supply),
                ))
            })?;

        Self::deposit_event(Event::RemoveLiquidity(
            provider,
            pool_id,
            token_amounts,
            fees,
            token_supply,
        ));

        Ok(().into())
    }

    fn remove_liquidity_imbalance(
        who: &Self::AccountId,
        pool_id: PoolId,
        amounts: Vec<Self::Balance>,
        max_burn_amount: Self::Balance,
    ) -> DispatchResultWithPostInfo {
        let zero = Self::get_number(0);

        let b_zero = Self::convert_number_to_balance(zero);
        ensure!(
            amounts.iter().all(|&x| x >= b_zero),
            Error::<T>::WrongAssetAmount
        );

        let (provider, pool_id, token_amounts, fees, invariant, token_supply, burn_amount) =
            Pools::<T>::try_mutate(pool_id, |pool| -> Result<_, DispatchError> {
                let pool = pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;

                let n_coins = pool.assets.len();

                ensure!(
                    n_coins == pool.balances.len(),
                    Error::<T>::InconsistentStorage
                );

                ensure!(n_coins == amounts.len(), Error::<T>::IndexOutOfRange);

                let ann = Self::get_ann(pool.amplification, n_coins).ok_or(Error::<T>::Math)?;

                let old_balances = Self::convert_vec_balance_to_number(pool.balances.clone());

                let d0 = Self::get_d(&old_balances, ann).ok_or(Error::<T>::Math)?;

                let mut new_balances = old_balances.clone();
                let n_amounts = amounts
                    .iter()
                    .copied()
                    .map(Self::convert_balance_to_number)
                    .collect::<Vec<_>>();
                for i in 0..n_coins {
                    new_balances[i] = new_balances[i]
                        .checked_sub(&n_amounts[i])
                        .ok_or(Error::<T>::Math)?;
                }

                let d1 = Self::get_d(&new_balances, ann).ok_or(Error::<T>::Math)?;

                // Deposit x + withdraw y would charge about same
                // fees as a swap. Otherwise, one could exchange w/o paying fees.
                // And this formula leads to exactly that equality
                // fee = pool.fee * n_coins / (4 * (n_coins - 1))
                let fee = (|| {
                    let n_coins =
                        <T::Convert as CheckedConvert<usize, T::Number>>::convert(n_coins)?;
                    let one = Self::get_number(1);
                    let four = one
                        .checked_add(&one)?
                        .checked_add(&one)?
                        .checked_add(&one)?;

                    <T::Convert as Convert<Permill, T::Number>>::convert(pool.fee)
                        .checked_mul(&n_coins)?
                        .checked_div(&four.checked_mul(&n_coins.checked_sub(&one)?)?)
                })()
                .ok_or(Error::<T>::Math)?;
                let admin_fee =
                    <T::Convert as Convert<Permill, T::Number>>::convert(pool.admin_fee);
                let mut fees = vec![zero; n_coins];
                for i in 0..n_coins {
                    let new_balance = new_balances[i];

                    ensure!(d0 != zero, Error::<T>::InsufficientFunds);
                    // ideal_balance = d1 * old_balances[i] / d0
                    let ideal_balance = (|| d1.checked_mul(&old_balances[i])?.checked_div(&d0))()
                        .ok_or(Error::<T>::Math)?;

                    // difference = abs(ideal_balance - new_balance)
                    let difference = (if ideal_balance > new_balance {
                        ideal_balance.checked_sub(&new_balance)
                    } else {
                        new_balance.checked_sub(&ideal_balance)
                    })
                    .ok_or(Error::<T>::Math)?;

                    fees[i] = fee.checked_mul(&difference).ok_or(Error::<T>::Math)?;

                    // pool.balances[i] = new_balance - (fees[i] * admin_fee)
                    pool.balances[i] = Self::convert_number_to_balance(
                        (|| new_balance.checked_sub(&fees[i].checked_mul(&admin_fee)?))()
                            .ok_or(Error::<T>::Math)?,
                    );

                    new_balances[i] = new_balances[i]
                        .checked_sub(&fees[i])
                        .ok_or(Error::<T>::Math)?;
                }
                let d2 = Self::get_d(&new_balances, ann).ok_or(Error::<T>::Math)?;

                let token_supply =
                    Self::convert_balance_to_number(T::Assets::total_issuance(pool.pool_asset));
                // token_amount = token_supply * (d0 - d2) / d0
                let token_amount = (|| {
                    token_supply
                        .checked_mul(&d0.checked_sub(&d2)?)?
                        .checked_div(&d0)
                })()
                .ok_or(Error::<T>::Math)?;
                ensure!(token_amount != zero, Error::<T>::WrongAssetAmount);
                // In case of rounding errors - make it unfavorable for the "attacker"
                let token_amount = token_amount
                    .checked_add(&T::Precision::get())
                    .ok_or(Error::<T>::Math)?;

                ensure!(
                    token_amount <= Self::convert_balance_to_number(max_burn_amount),
                    Error::<T>::RequiredAmountNotReached
                );

                let new_token_supply = token_supply
                    .checked_sub(&token_amount)
                    .ok_or(Error::<T>::Math)?;

                let burn_amount = Self::convert_number_to_balance(token_amount);

                T::Assets::burn(pool.pool_asset, &who, burn_amount)?;

                // Ensure that for all tokens we have sufficient amount
                for i in 0..n_coins {
                    ensure!(
                        T::Assets::balance(pool.assets[i], &T::PalletId::get().into_account())
                            >= amounts[i],
                        Error::<T>::InsufficientFunds
                    );
                }

                for i in 0..n_coins {
                    if n_amounts[i] > zero {
                        Self::transfer_liquidity_from_pool(pool, i, &who, amounts[i])?;
                    }
                }

                let fees = fees
                    .into_iter()
                    .map(|x| Self::convert_number_to_balance(x))
                    .collect::<Vec<T::Balance>>();

                Ok((
                    who.clone(),
                    pool_id,
                    amounts,
                    fees,
                    Self::convert_number_to_balance(d1),
                    Self::convert_number_to_balance(new_token_supply),
                    burn_amount,
                ))
            })?;

        Self::deposit_event(Event::RemoveLiquidityImbalance(
            provider,
            pool_id,
            token_amounts,
            fees,
            invariant,
            token_supply,
            burn_amount,
        ));

        Ok(().into())
    }

    fn remove_liquidity_one_coin(
        who: &Self::AccountId,
        pool_id: PoolId,
        token_amount: Self::Balance,
        i: PoolTokenIndex,
        min_amount: Self::Balance,
    ) -> DispatchResultWithPostInfo {
        let pti_i = i;
        let i = i as usize;

        let b_zero = Self::convert_number_to_balance(Self::get_number(0));
        ensure!(token_amount >= b_zero, Error::<T>::WrongAssetAmount);

        let n_token_amount = Self::convert_balance_to_number(token_amount);

        let (provider, pool_id, burn_amount, dy, new_token_supply, dy_fee) =
            Pools::<T>::try_mutate(pool_id, |pool| -> Result<_, DispatchError> {
                let pool = pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;

                let n_coins = pool.assets.len();

                ensure!(
                    n_coins == pool.balances.len(),
                    Error::<T>::InconsistentStorage
                );
                ensure!(i < n_coins, Error::<T>::IndexOutOfRange);

                let ann = Self::get_ann(pool.amplification, n_coins).ok_or(Error::<T>::Math)?;

                let token_supply =
                    Self::convert_balance_to_number(T::Assets::total_issuance(pool.pool_asset));
                let pool_fee = <T::Convert as Convert<Permill, T::Number>>::convert(pool.fee);

                let pool_balances = Self::convert_vec_balance_to_number(pool.balances.clone());
                let (dy, dy_fee) = Self::calc_withdraw_one_coin(
                    n_token_amount,
                    i,
                    &pool_balances,
                    ann,
                    token_supply,
                    pool_fee,
                )
                .ok_or(Error::<T>::Math)?;

                ensure!(
                    dy > Self::convert_balance_to_number(min_amount),
                    Error::<T>::RequiredAmountNotReached
                );

                let admin_fee =
                    <T::Convert as Convert<Permill, T::Number>>::convert(pool.admin_fee);

                // pool.balances[i] = pool.balances[i] - (dy + dy_fee * pool.admin_fee)
                pool.balances[i] = Self::convert_number_to_balance(
                    (|| {
                        pool_balances[i]
                            .checked_sub(&dy.checked_add(&dy_fee.checked_mul(&admin_fee)?)?)
                    })()
                    .ok_or(Error::<T>::Math)?,
                );

                let new_token_supply = token_supply
                    .checked_add(&n_token_amount)
                    .ok_or(Error::<T>::Math)?;

                let b_dy = Self::convert_number_to_balance(dy);

                ensure!(
                    T::Assets::balance(pool.assets[i], &T::PalletId::get().into_account()) >= b_dy,
                    Error::<T>::InsufficientFunds
                );

                T::Assets::burn(pool.pool_asset, &who, token_amount)?;

                Self::transfer_liquidity_from_pool(pool, i, &who, b_dy)?;

                Ok((
                    who.clone(),
                    pool_id,
                    token_amount,
                    b_dy,
                    Self::convert_number_to_balance(new_token_supply),
                    Self::convert_number_to_balance(dy_fee),
                ))
            })?;

        Self::deposit_event(Event::RemoveLiquidityOne(
            provider,
            pool_id,
            burn_amount,
            pti_i,
            dy,
            new_token_supply,
            dy_fee,
        ));

        Ok(().into())
    }

    fn get_dy(
        pool_id: PoolId,
        i: PoolTokenIndex,
        j: PoolTokenIndex,
        dx: Self::Balance,
    ) -> Result<Self::Balance, DispatchError> {
        Pallet::<T>::get_dy(pool_id, i, j, dx)
    }

    fn get_virtual_price(pool_id: PoolId) -> Result<Self::Balance, DispatchError> {
        Pallet::<T>::get_virtual_price(pool_id)
    }

    fn withdraw_admin_fees(who: &Self::AccountId, pool_id: PoolId) -> DispatchResultWithPostInfo {
        let admin_fees =
            Pools::<T>::try_mutate(pool_id, |maybe_pool| -> Result<_, DispatchError> {
                let pool = maybe_pool.as_mut().ok_or(Error::<T>::PoolNotFound)?;
                let n_coins = pool.assets.len();

                ensure!(
                    n_coins == pool.balances.len(),
                    Error::<T>::InconsistentStorage
                );

                let total_balances = &pool.total_balances;
                let balances = &pool.balances;

                let admin_fees = total_balances
                    .into_iter()
                    .zip(balances)
                    .map(|(tb, b)| {
                        let admin_fee = tb.checked_sub(b).ok_or(Error::<T>::Math)?;

                        Ok(admin_fee)
                    })
                    .collect::<Result<Vec<Self::Balance>, DispatchError>>()?;

                T::Assets::withdraw_admin_fees(pool_id, admin_fees.iter().copied())?;

                for i in 0..n_coins {
                    pool.total_balances[i] = pool.total_balances[i]
                        .checked_sub(&admin_fees[i])
                        .ok_or(Error::<T>::Math)?;
                }

                Ok(admin_fees)
            })?;

        Self::deposit_event(Event::WithdrawAdminFees(who.clone(), pool_id, admin_fees));

        Ok(().into())
    }
}

/// Module that contain traits which must be implemented somewhere in the runtime
/// in order to equilibrium_curve_amm pallet can work properly.
pub mod traits {
    use crate::{PoolId, PoolInfo, PoolTokenIndex};
    use frame_support::dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo};
    use impl_trait_for_tuples::impl_for_tuples;
    use sp_runtime::Permill;
    use sp_std::vec::Vec;

    /// Pallet equilibrium_curve_amm should interact with custom Assets.
    /// In order to do this it relies on `Assets` trait implementation.
    pub trait Assets<AssetId, Balance, AccountId> {
        /// Creates new asset
        fn create_asset(pool_id: PoolId) -> Result<AssetId, DispatchError>;
        /// Mint tokens for the specified asset
        fn mint(asset: AssetId, dest: &AccountId, amount: Balance) -> DispatchResult;
        /// Burn tokens for the specified asset
        fn burn(asset: AssetId, dest: &AccountId, amount: Balance) -> DispatchResult;
        /// Transfer tokens for the specified asset
        fn transfer(
            asset: AssetId,
            source: &AccountId,
            dest: &AccountId,
            amount: Balance,
        ) -> DispatchResult;
        /// Withdraw admin fees
        fn withdraw_admin_fees(
            pool_id: PoolId,
            amounts: impl Iterator<Item = Balance>,
        ) -> DispatchResult;
        /// Checks the balance for the specified asset
        fn balance(asset: AssetId, who: &AccountId) -> Balance;
        /// Returns total issuance of the specified asset
        fn total_issuance(asset: AssetId) -> Balance;
        /// For benchmarking purposes only
        #[cfg(feature = "runtime-benchmarks")]
        fn create_benchmark_asset() -> AssetId;
    }

    /// Generic conversion trait. Unlike `sp_runtime::traits::Convert` it supports cases
    /// where some values of type `A` can not be represented in type `B`.
    pub trait CheckedConvert<A, B> {
        /// Make a conversion
        fn convert(a: A) -> Option<B>;
    }

    /// Provides functionality of the `equilibrium-curve-amm` pallet for other pallets.
    pub trait CurveAmm {
        /// The asset ID type
        type AssetId;
        /// The number type for underlying calculations
        type Number;
        /// The balance type of an account
        type Balance;
        /// The user account identifier type for the runtime
        type AccountId;

        /// Current number of pools (also ID for the next created pool)
        fn pool_count() -> PoolId;

        /// Information about the pool with the specified `id`
        fn pool(
            id: PoolId,
        ) -> Option<PoolInfo<Self::AccountId, Self::AssetId, Self::Number, Self::Balance>>;

        /// Creates a pool, taking a creation fee from the caller
        fn create_pool(
            who: &Self::AccountId,
            assets: Vec<Self::AssetId>,
            amplification: Self::Number,
            fee: Permill,
            admin_fee: Permill,
        ) -> DispatchResultWithPostInfo;

        /// Deposit coins into the pool
        /// `amounts` - list of amounts of coins to deposit,
        /// `min_mint_amount` - minimum amout of LP tokens to mint from the deposit.
        fn add_liquidity(
            who: &Self::AccountId,
            pool_id: PoolId,
            amounts: Vec<Self::Balance>,
            min_mint_amount: Self::Balance,
        ) -> DispatchResultWithPostInfo;

        /// Perform an exchange between two coins.
        /// `i` - index value of the coin to send,
        /// `j` - index value of the coin to recieve,
        /// `dx` - amount of `i` being exchanged,
        /// `min_dy` - minimum amount of `j` to receive.
        fn exchange(
            who: &Self::AccountId,
            pool_id: PoolId,
            i: PoolTokenIndex,
            j: PoolTokenIndex,
            dx: Self::Balance,
            min_dy: Self::Balance,
        ) -> DispatchResultWithPostInfo;

        /// Withdraw coins from the pool.
        /// Withdrawal amount are based on current deposit ratios.
        /// `amount` - quantity of LP tokens to burn in the withdrawal,
        /// `min_amounts` - minimum amounts of underlying coins to receive.
        fn remove_liquidity(
            who: &Self::AccountId,
            pool_id: PoolId,
            amount: Self::Balance,
            min_amounts: Vec<Self::Balance>,
        ) -> DispatchResultWithPostInfo;

        /// Withdraw coins from the pool in an imbalanced amount.
        /// `amounts` - list of amounts of underlying coins to withdraw,
        /// `max_burn_amount` - maximum amount of LP token to burn in the withdrawal.
        fn remove_liquidity_imbalance(
            who: &Self::AccountId,
            pool_id: PoolId,
            amounts: Vec<Self::Balance>,
            max_burn_amount: Self::Balance,
        ) -> DispatchResultWithPostInfo;

        /// Withdraw a single coin from the pool.
        /// `token_amount` - amount of LP tokens to burn in the withdrawal,
        /// `i` - index value of the coin to withdraw,
        /// `min_amount` - minimum amount of coin to receive.
        fn remove_liquidity_one_coin(
            who: &Self::AccountId,
            pool_id: PoolId,
            token_amount: Self::Balance,
            i: PoolTokenIndex,
            min_amount: Self::Balance,
        ) -> DispatchResultWithPostInfo;

        /// Calculates the exchange outcome `dy` for a given `i`, `j` and `dx` values.
        fn get_dy(
            pool_id: PoolId,
            i: PoolTokenIndex,
            j: PoolTokenIndex,
            dx: Self::Balance,
        ) -> Result<Self::Balance, DispatchError>;

        /// The current virtual price of the pool LP token.
        fn get_virtual_price(pool_id: PoolId) -> Result<Self::Balance, DispatchError>;

        /// Withdraw admin fees
        fn withdraw_admin_fees(
            who: &Self::AccountId,
            pool_id: PoolId,
        ) -> DispatchResultWithPostInfo;
    }

    /// Notification about new pool created.
    pub trait OnPoolCreated {
        fn on_pool_created(pool_id: PoolId);
    }

    #[impl_for_tuples(5)]
    impl OnPoolCreated for Tuple {
        fn on_pool_created(pool_id: PoolId) {
            for_tuples!( #( Tuple::on_pool_created(pool_id); )* );
        }
    }

    /// Checks if all items in the slice are met some condition
    pub trait SliceChecker<T> {
        /// Returns `()` if all `items` are met some condition and `DispatchError` otherwise
        fn check(items: &[T]) -> Result<(), DispatchError>;
    }

    impl<T> SliceChecker<T> for () {
        fn check(_items: &[T]) -> Result<(), DispatchError> {
            Ok(())
        }
    }

    /// Special trait for runtime specific initialization
    #[cfg(feature = "runtime-benchmarks")]
    pub trait BenchmarkingInit {
        /// Initialization of a runtime before `withdraw_admin_fees` call
        fn init_withdraw_admin_fees() {}
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl BenchmarkingInit for () {}
}

/// Type that represents index type of token in the pool passed from the outside as an extrinsic
/// argument.
pub type PoolTokenIndex = u32;

/// Type that represents pool id
pub type PoolId = u32;

/// Storage record type for a pool
#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug, scale_info::TypeInfo)]
pub struct PoolInfo<AccountId, AssetId, Number, Balance> {
    /// Owner of pool
    pub owner: AccountId,
    /// LP multiasset
    pub pool_asset: AssetId,
    /// List of multiassets supported by the pool
    pub assets: Vec<AssetId>,
    /// Initial amplification coefficient (leverage)
    pub amplification: Number,
    /// Amount of the fee pool charges for the exchange
    pub fee: Permill,
    /// Amount of the admin fee pool charges for the exchange
    pub admin_fee: Permill,
    /// Current balances excluding admin_fee
    pub balances: Vec<Balance>,
    /// Current balances including admin_fee
    pub total_balances: Vec<Balance>,
}
