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

#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

//mod math;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::codec::{Decode, Encode};
use sp_runtime::Permill;
use sp_std::prelude::*;

#[frame_support::pallet]
pub mod pallet {
    use super::{traits::Assets, PoolInfo};
    use frame_support::{
        dispatch::{Codec, DispatchResult, DispatchResultWithPostInfo},
        pallet_prelude::*,
        traits::{Currency, ExistenceRequirement, OnUnbalanced, WithdrawReasons},
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::{ModuleId, Permill};
    use sp_std::collections::btree_set::BTreeSet;
    use sp_std::iter::FromIterator;
    use sp_std::prelude::*;
    use substrate_fixed::traits::Fixed;

    /// Config of Equilibrium Curve Amm pallet
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Identificator type of Asset
        type AssetId: Parameter + Ord + Copy;
        /// The balance of an account
        type Balance: Encode;
        /// External implementation for required opeartions with assets
        type Assets: super::traits::Assets<Self::AssetId, Self::Balance, Self::AccountId>;
        /// Standart balances pallet for utility token or adapter
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
        type ModuleId: Get<ModuleId>;

        /// Number type for underlying calculations
        type Number: Parameter + From<Permill>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Current pools count
    #[pallet::storage]
    #[pallet::getter(fn pool_count)]
    pub type PoolCount<T: Config> = StorageValue<_, u32, ValueQuery>;

    /// All pools infos
    #[pallet::storage]
    #[pallet::getter(fn pools)]
    pub type Pools<T: Config> =
        StorageMap<_, Blake2_128Concat, u32, PoolInfo<T::AssetId, T::Number>>;

    /// Event type for Equilibrium Curve AMM pallet
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Pool with specified id created successfully
        PoolCreated(T::AccountId, u32),
    }

    /// Error type for Equilibrium Curve AMM pallet
    #[pallet::error]
    pub enum Error<T> {
        /// Could not create new asset
        AssetNotCreated,
        /// User does not have required amount of currency to complete operation
        NotEnoughForFee,
        /// Values in the storage are inconsistent
        InconsistentStorage,
        /// Not enough assets provided
        NotEnoughAssets,
        /// Some provided assets are not unique
        DuplicateAssets,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Creates pool, taking creation fee from the caller
        #[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
        pub fn create_pool(
            origin: OriginFor<T>,
            assets: Vec<T::AssetId>,
            amplification: T::Number,
            fee: Permill,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // Assets related checks
            ensure!(assets.len() > 1, Error::<T>::NotEnoughAssets);
            let unique_assets = BTreeSet::<T::AssetId>::from_iter(assets.iter().copied());
            ensure!(
                unique_assets.len() == assets.len(),
                Error::<T>::DuplicateAssets
            );

            // Take fee
            let creation_fee = T::CreationFee::get();
            let imbalance = T::Currency::withdraw(
                &who,
                creation_fee,
                WithdrawReasons::FEE,
                ExistenceRequirement::AllowDeath,
            )
            .map_err(|_| Error::<T>::NotEnoughForFee)?;
            T::OnUnbalanced::on_unbalanced(imbalance);

            // Add new pool
            let pool_key =
                PoolCount::<T>::try_mutate(|pool_count| -> Result<u32, DispatchError> {
                    let pool_key = *pool_count;

                    Pools::<T>::try_mutate_exists(pool_key, |maybe_pool_info| -> DispatchResult {
                        // We expect that PoolInfos have sequential keys.
                        // No PoolInfo can have key greater or equal to PoolCount
                        maybe_pool_info
                            .as_ref()
                            .map(|_| Err(Error::<T>::InconsistentStorage))
                            .unwrap_or(Ok(()))?;

                        let asset =
                            T::Assets::create_asset().map_err(|_| Error::<T>::AssetNotCreated)?;

                        *maybe_pool_info = Some(PoolInfo {
                            pool_asset: asset,
                            assets: assets,
                            amplification,
                            fee,
                        });

                        Ok(())
                    })?;

                    *pool_count = pool_key
                        .checked_add(1)
                        .ok_or(Error::<T>::InconsistentStorage)?;

                    Ok(pool_key)
                })?;

            Self::deposit_event(Event::PoolCreated(who.clone(), pool_key));

            Ok(().into())
        }
    }
}

pub mod traits {
    use frame_support::dispatch::{DispatchError, DispatchResult};

    /// We need to operate with custom Assets, so we will create this trait and simple
    /// implementation for it. Other projects can add adapters to adapt their realization.
    pub trait Assets<AssetId, Balance, AccountId> {
        /// Creates new asset
        fn create_asset() -> Result<AssetId, DispatchError>;
        /// Mint tokens for the specified asset
        fn mint(asset: AssetId, dest: AccountId, amount: Balance) -> DispatchResult;
        /// Burn tokens for the specified asset
        fn burn(asset: AssetId, dest: AccountId, amount: Balance) -> DispatchResult;
        /// Transfer tokens for the specified asset
        fn transfer(
            asset: AssetId,
            source: AccountId,
            dest: AccountId,
            amount: Balance,
        ) -> DispatchResult;
        /// Checks the balance for the specified asset
        fn balance(asset: AssetId, who: AccountId) -> Balance;
        /// Returns total issuance of the specified asset
        fn total_issuance(asset: AssetId) -> Balance;
    }
}

/// Storage record type for a pool
#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
pub struct PoolInfo<AssetId, Number> {
    /// LP multiasset
    pool_asset: AssetId,
    /// List of multiassets supported by the pool
    assets: Vec<AssetId>,
    /// Initial amplification coefficient (leverage)
    amplification: Number,
    /// Amount of the fee pool charges for the exchange
    fee: Permill,
}

use sp_runtime::traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub};
use sp_std::cmp::Ordering;
use sp_std::convert::TryFrom;

pub struct MathError;

pub type PrimitiveType = u16;

pub fn get_d<N>(xp: &[N], amp: N) -> Option<N>
where
    N: CheckedAdd
        + CheckedSub
        + CheckedMul
        + CheckedDiv
        + From<(PrimitiveType, PrimitiveType)>
        + Copy
        + Eq
        + Ord,
{
    let zero = N::from((PrimitiveType::from(0u8), PrimitiveType::from(1u8)));
    let one = N::from((PrimitiveType::from(1u8), PrimitiveType::from(1u8)));
    let ten = N::from((PrimitiveType::from(10u8), PrimitiveType::from(1u8)));

    //TODO temp code
    let mut prec = one;
    for ii in 0..6 {
        prec = prec.checked_div(&ten)?;
    }

    let n_coins = N::from((
        PrimitiveType::try_from(xp.len()).ok()?,
        PrimitiveType::from(1u8),
    ));

    let mut s = zero;
    let mut d_prev = zero;

    for x in xp.iter() {
        s = s.checked_add(x)?;
    }
    if s == zero {
        return None;
    }

    let mut d = s;

    // ann = amp * n^n
    let mut ann = amp;
    for i in 0..xp.len() {
        ann = ann.checked_mul(&n_coins)?;
    }

    for i in 0..255 {
        let mut d_p = d;
        for x in xp.iter() {
            // d_p = d_p * d / (x * n_coins)
            d_p = d_p
                .checked_mul(&d)?
                .checked_div(&x.checked_mul(&n_coins)?)?;
        }
        d_prev = d;
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

        if d.cmp(&d_prev) == Ordering::Greater {
            if d.checked_sub(&d_prev)?.cmp(&prec) != Ordering::Greater {
                return Some(d);
            }
        } else {
            if d_prev.checked_sub(&d)?.cmp(&prec) != Ordering::Greater {
                return Some(d);
            }
        }
    }

    None
}

/// Calculate x[j] if on makes x[i] = x
///
/// Done by solving quadratic equation iteratively.
/// x1^2 + x1 * (sum' - (A*n^n - 1) * D / (A * n^n)) = D^(n + 1) / (n^(2*n) * prod' * A)
/// x1^2 + b * x1 = c
/// x1 = (x1^2 + c) / (2 * x1 + b)
pub fn get_y<N>(i: PrimitiveType, j: PrimitiveType, x: N, xp: &[N], amp: N) -> Option<N>
where
    N: CheckedAdd
        + CheckedSub
        + CheckedMul
        + CheckedDiv
        + From<(PrimitiveType, PrimitiveType)>
        + Copy
        + Eq
        + Ord,
{
    let zero = N::from((PrimitiveType::from(0u8), PrimitiveType::from(1u8)));
    let one = N::from((PrimitiveType::from(1u8), PrimitiveType::from(1u8)));
    let two = N::from((PrimitiveType::from(2u8), PrimitiveType::from(1u8)));
    let ten = N::from((PrimitiveType::from(10u8), PrimitiveType::from(1u8)));

    //TODO temp code
    let mut prec = one;
    for ii in 0..6 {
        prec = prec.checked_div(&ten)?;
    }

    let i_us = usize::from(i);
    let j_us = usize::from(j);

    let n_coins = N::from((
        PrimitiveType::try_from(xp.len()).ok()?,
        PrimitiveType::from(1u8),
    ));

    // Same coin
    if !(i != j) {
        return None;
    }
    // j below zero
    if !(j >= 0) {
        return None;
    }
    // j above n_coins
    if !(j_us < xp.len()) {
        return None;
    }

    // Should be unreachable, but good for safety
    if !(i >= 0) {
        return None;
    }
    if !(i_us < xp.len()) {
        return None;
    }

    let d = get_d(xp, amp)?;
    // ann = amp * n^n
    let mut ann = amp;
    for i in 0..xp.len() {
        ann = ann.checked_mul(&n_coins)?;
    }
    let mut c = d;
    let mut s = zero;
    let mut xx = zero;
    let mut y_prev = zero;

    for ii in 0..xp.len() {
        if ii == i_us {
            xx = x;
        } else if ii != j_us {
            xx = xp[ii];
        } else {
            continue;
        }
        // s = s + xx
        s = s.checked_add(&xx)?;
        // c = c * d / (xx * n_coins)
        let c_prev = c;
        c = c.checked_mul(&d)?.checked_div(&xx.checked_mul(&n_coins)?)?;
    }
    // c = c * d / (ann * n_coins)
    c = c
        .checked_mul(&d)?
        .checked_div(&ann.checked_mul(&n_coins)?)?;

    // b = s + d / ann // - d
    let b = s.checked_add(&d.checked_div(&ann)?)?;
    let mut y = d;

    for ii in 0..255 {
        y_prev = y;
        // y = (y^2 + c) / (2 * y + b - d)
        y = y
            .checked_mul(&y)?
            .checked_add(&c)?
            .checked_div(&two.checked_mul(&y)?.checked_add(&b)?.checked_sub(&d)?)?;
        // Equality with the precision of 1
        if y.cmp(&y_prev) == Ordering::Greater {
            if y.checked_sub(&y_prev)?.cmp(&prec) != Ordering::Greater {
                return Some(y);
            }
        } else {
            if y_prev.checked_sub(&y)?.cmp(&prec) != Ordering::Greater {
                return Some(y);
            }
        }
    }

    None
}

#[cfg(test)]
mod math_tests {
    use super::*;
    use sp_runtime::traits::Saturating;
    use sp_runtime::{FixedI128, FixedPointNumber};

    #[test]
    fn get_d_impl() {
        let result = get_d(
            &vec![
                FixedI128::saturating_from_rational(11, 10),
                FixedI128::saturating_from_rational(88, 100),
            ],
            FixedI128::saturating_from_rational(292, 100),
        );

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
    fn get_y_impl() {
        let result = get_y(
            0,
            1,
            FixedI128::saturating_from_rational(111, 100),
            &vec![
                FixedI128::saturating_from_rational(11, 10),
                FixedI128::saturating_from_rational(88, 100),
            ],
            FixedI128::saturating_from_rational(292, 100),
        );

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
}
