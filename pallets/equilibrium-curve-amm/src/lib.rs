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
    use super::traits::Assets;
    use frame_support::{
        dispatch::{Codec, DispatchResultWithPostInfo},
        pallet_prelude::*,
    };
    use frame_system::pallet_prelude::*;
    use sp_runtime::Permill;
    use sp_std::prelude::*;
    use substrate_fixed::traits::Fixed;

    /// Config of Equilibrium Curve Amm pallet
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Identificator type of Asset
        type AssetId: Parameter;
        /// The balance of an account
        type Balance: Encode;
        /// External implementation for required opeartions with assets
        type Assets: super::traits::Assets<Self::AssetId, Self::Balance, Self::AccountId>;
        /// Standart balances pallet for utility token or adapter
        type Currency;
        /// Anti ddos fee for pool creation
        #[pallet::constant]
        type CreationFee: Get<Self::Balance>;
        //type OnUnbalanced: OnUnbalanced;
        //type ModuleId: ModuleId;
        /// Number type for underlying calculations
        type Number: Parameter + From<Permill>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// All pools infos
    #[pallet::storage]
    #[pallet::getter(fn pools)]
    pub type Pools<T: Config> = StorageValue<_, super::PoolInfo<T::AssetId, T::Number>>;

    /// Event type for Equilibrium Curve AMM pallet
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Temporary stub event
        StubEvent(u32, T::AccountId),
    }

    /// Error type for Equilibrium Curve AMM pallet
    #[pallet::error]
    pub enum Error<T> {
        /// Could not create new asset
        AssetNotCreated,
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
            let _who = ensure_signed(origin)?;

            // take fee
            // add new pool

            // todo: make error value meaningful
            let asset = T::Assets::create_asset().map_err(|_| Error::<T>::AssetNotCreated)?;

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
    PoolAsset: AssetId,
    /// List of multiassets supported by the pool
    Assets: Vec<AssetId>,
    /// Initial amplification coefficient (leverage)
    Amplification: Number,
    /// Amount of the fee pool charges for the exchange
    Fee: Permill,
}
