# Guide

This guide is divided into two parts:

- See [Adding the Pallet](#adding-the-pallet) for information on how to integrate `equilibrium-curve-amm` pallet into your chain.
- See [Code Examples](#code-examples) for how to use the pallet when it up and running.

## Adding the Pallet

The `equilibrium-curve-amm` pallet require you to provide several types that all fall into four parts: balance, assets, pool, math.

Balance-related types:

- `Balance` - the balance of an account in your chain. This type also represents asset amounts.
  Out-of-the-box `node-template` project declares its balance type as `u128`,
  and that's pretty good choice.
  Nevertheless, the balance type is altered to `u64` in the node runtime distributed with `equilibrium-curve-amm` pallet.
- `Currency` - should be set to `pallet_balances::Pallet<Runtime>` in most cases. Can be altered to something else in case your chain uses a custom balances system and not substrate's default `pallet_balances`.
  
Assets related types:

- `Assets` - `equilibrium-curve-amm` pallet uses this to communicate with the asset system.
  The chosen asset system should be capable of creating new assets, minting, burning, and transferring a specified amount of existing assets, providing information about the total issuance and balance of the particular account.
  Node runtime distributed with this pallet contains the implementation of an `Assets` trait for substrate's `pallet-assets`. But you should consider it as a starting point for your own implementation, rather than a complete production-ready solution.
- `AssetId` - the asset identifier type used by the chosen asset system. As an example the node runtime distributed with this pallet uses `u32`.


Pool related types:

- `CreationFee` - new pool creation fee. It's withdrawn in a chain's currency rather than some other asset.
- `OnUnbalanced` - this type decides what to do with the fee charged for the pool creation.
- `ModuleId` - the parameter that identifies the `equilibrium-curve-amm` pallet itself. The LP-asset of every pool is backed by appropriate underlying pool assets. These assets are deposited to the `equilibrium-curve-amm` account that can be accessed by its `ModuleId`.

Math related types:

- `Number` - the type for internal math calculations.
  All number values supplied to the `equilibrium-curve-amm` are converted into this type. After performing all the necessary math, the results are converted back. 
  As an example, node runtime distributed with this pallet uses `FixedU128`.
  Note that it's perfectly fine if `Number` and `Balance` types are identical.
- `Precision` - a parameter that defines fixed-point iteration method precision.
  The internal pallet's functions are such that `get_y`, `get_d` and `get_y_d` depend on it.
  In case `Number` type and `Balance` type are the same in your chain, you can just simply set this parameter to 1.
- `Convert` - a type for performing conversions from external numeric values into internal `Number` type, and vice versa.

Do not forget to look at [lib.rs](../runtime/src/lib.rs) file, as it contains a fully functional example of `equilibrium-curve-amm` pallet integration into the node. 

Let's consider the most important code snippets from this file.

### Parameters

Here we declare all the parameters that the `equilibrium-curve-amm` pallet needs:

```rust
parameter_types! {
    pub const CreationFee: Balance = 999;
    pub const CurveAmmModuleId: ModuleId = ModuleId(*b"eq/crvam");
    pub Precision: FixedU128 = FixedU128::saturating_from_rational(1, 1_000_000);
}
```

The `CreationFee` parameter can be 0. In this case a fee will not be charged for the pool creation operation.

It is safe to leave `CurveAmmModuleId` parameter unchanged.

The `Precision` parameter value equals 1e-6 in this case.
But as mentioned above, in most cases it should be set to the least value possible for the `Number` type.
So if you set both `Number` type and `Balance` type to be `u128`, you can set `Precision` to `1u128`. 

### Conversion Routines

The `equilibrium-curve-amm` pallet uses various substrate types as input and output.
But all math calculations internally it does using `Number` type. So you should write all required conversion functions: 

- Conversion from `Balance` to `Number`. Both currency and asset amounts are represented by `Balance` type.
- Conversion from `Number` to `Balance`. All calculation results are asset amounts.
- Conversion from substrate type `sp_arithmetic::per_things::Permill`. Fee and admin fee are represented by this type.
- Conversion from `u8` to `Number`. Values of `u8` types are used to construct small constants (such that 0, 1, 2) in terms of `Number` type. They are used extensively in the associated number crunching. 
- Conversion from `usize` to `Number`. Values of `usize` represent the length of vectors in terms of `Number` type.

Note that the node runtime distributed with this pallet declares `Number` type as `type Number = FixedU128`.
So these two types can be used interchangeably is runtime's `lib.rs` code.

The following  snippet of code contains all required conversions from this file:   

```rust
pub struct FixedU128Convert;

impl Convert<Permill, FixedU128> for FixedU128Convert {
    fn convert(a: Permill) -> FixedU128 {
        a.into()
    }
}

impl Convert<Balance, FixedU128> for FixedU128Convert {
    fn convert(a: Balance) -> FixedU128 {
        let accuracy = FixedU128::accuracy() / FixedI64::accuracy() as u128;
        FixedU128::from_inner(a as u128 * accuracy)
    }
}

impl Convert<u8, FixedU128> for FixedU128Convert {
    fn convert(a: u8) -> FixedU128 {
        FixedU128::saturating_from_integer(a)
    }
}

impl equilibrium_curve_amm::traits::CheckedConvert<usize, FixedU128> for FixedU128Convert {
    fn convert(a: usize) -> Option<FixedU128> {
        Some(FixedU128::saturating_from_integer(u128::try_from(a).ok()?))
    }
}

impl Convert<FixedU128, Balance> for FixedU128Convert {
    fn convert(a: FixedU128) -> Balance {
        let accuracy = FixedU128::accuracy() / FixedI64::accuracy() as u128;
        // NOTE: Please do not use `unwrap` function in production.
        (a.into_inner() / accuracy).try_into().unwrap()
    }
}
```

Any strange looking conversions from `Balance` to `FixedU128` are due to how `Balance` value is stored.
It's format is the same as `FixedI64` format with the only difference that `FixedI64` is signed but `Balance` is not.

Anyway, in most cases when `Balance` and `Number` types are the same, all of these conversions are trivial.

### Asset Backend

Unlike the previous example of code, this one isn't from the runtime's `lib.rs` file.
Here is just a definition of an `Assets` trait that each chain should implement in order to use `equilibrium-curve-amm` pallet:

```rust
/// Pallet equilibrium_curve_amm should interact with custom Assets.
/// In order to do this it relies on `Assets` trait implementation.
pub trait Assets<AssetId, Balance, AccountId> {
    /// Creates new asset
    fn create_asset() -> Result<AssetId, DispatchError>;
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
    /// Checks the balance for the specified asset
    fn balance(asset: AssetId, who: &AccountId) -> Balance;
    /// Returns total issuance of the specified asset
    fn total_issuance(asset: AssetId) -> Balance;
}
```

See `FrameAssets` struct from [lib.rs](../runtime/src/lib.rs) for an example of implementation of the `Assets` trait.
(It uses the substrate `pallet-assets` pallet as an underlying assets system.)

But here are a few notes about this example implementation. Substrate's `pallet-assets` is not fully compatible with the `Assets` trait,
which expects that underlying asset system is able to generate an asset ID for every created asset on it's own.
Substrate's `pallet-assets` can't do this, and expects the asset ID to come from somewhere else.
To solve this problem, the `Assets` example implementation randomly generates an asset ID and tries to create a new asset using it.
This approach makes good hypothetical example but shouldn't actually be used in a production environment.

To fix this you can either write your own asset system implementation from scratch,
or you can fork substrate's `pallet-assets` and add an asset ID generation functionality to it.

### Implement Config Trait

As for now, all required parameters and types are prepared.
So it doesn't take much effort to implement `equilibrium_curve_amm::Config` trait for our Runtime: 

```rust
impl equilibrium_curve_amm::Config for Runtime {
    type Event = Event;
    type AssetId = AssetId;
    type Balance = Balance;
    type Currency = pallet_balances::Pallet<Runtime>;
    type CreationFee = CreationFee;
    type Assets = FrameAssets;
    type OnUnbalanced = EmptyUnbalanceHandler;
    type ModuleId = CurveAmmModuleId;

    type Number = Number;
    type Precision = Precision;
    type Convert = FixedU128Convert;
}
```

### Construct Runtime

The last step in adding a pallet to your chain is to register it in the `construct_runtime` macro.
In the follow code snippet, `equilibrium-curve-amm` pallet is added to the runtime as `CurveAmm` in the last line:

```rust
construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Module, Call, Config, Storage, Event<T>},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
        Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
        Aura: pallet_aura::{Module, Config<T>},
        Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},
        Assets: pallet_assets::{Module, Call, Storage, Event<T>},
        Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
        TransactionPayment: pallet_transaction_payment::{Module, Storage},
        Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},
        
        CurveAmm: equilibrium_curve_amm::{Module, Call, Storage, Event<T>},
    }
);
```

## Code examples

See [index.js](../client/index.js) for the full code. The client is written in JavaScript, as is all the subsequent code snippets.
Please read the client's dedicated [README.md](../client/README.md) before running it.

### Create Pool

Before creating a pool, you must create assets.
Use your underlying asset system extrinsics to do so.

Provide `createPool` extrinsic with an array of asset ID, amplification coefficient, fee and admin fee:

```javascript
api.tx.curveAmm.createPool(assetIds, amplification, fee, adminFee)
```

### Add Liquidity

Please note that initial liquidity deposit should be done for all assets in the pool.

All subsequent liquidity deposits may contain only some assets.

Asset amounts must form an array that preserves order of assets passed to `createPool` extrinsic.

Suppose, you want to deposit 100 units of an asset with ID of 33 and 50 units of asset with ID 44
and you know that a pool was created with an `assetIds` argument equal to `[33, 44]`.
So you should pass amounts to `addLiquidity` extrinsic as `[100, 50]`.

In case you do not want to deposit liquidity from some asset, you should pass 0 in its place in the array.
For example, you want to deposit only 50 units of asset with id 44.
It means that `addLiquidity` extrinsic should be called with `assetAmounts` parameter equals to `[0, 50]`.

```javascript
api.tx.curveAmm.addLiquidity(poolId, assetAmounts, minMintAmount)
```

### Exchange

Note that this extrinsic expects asset indices and not asset IDs.

For example, you want to exchange asset with an ID of 33 to an asset with the ID 55.
And you know that the `createPool` extrinsic was called with `assetIds` equal to `[11, 22, 33, 44, 55, 66]`.
So you should pass to the `exchange` extrinsic `fromAsset` equal to `2` and `toAsset` equal to `4`.

```javascript
api.tx.curveAmm.exchange(poolId, fromAssetIndex, toAssetIndex, fromAssetAmount, toAssetMinAmount)
```

### Remove Liquidity

There are three ways to remove liquidity from the pool.

The balanced way:
your removal will not change the StableSwap invariant.

```javascript
api.tx.curveAmm.removeLiquidity(poolId, lpAssetAmount, minAssetAmounts)
```

But also you can remove liquidity with more freedom:

```javascript
api.tx.curveAmm.removeLiquidityImbalance(poolId, assetAmounts, lpMaxBurnAmount)
```

Finally you can remove liquidity only for single asset.
But note that this extrinsic (like [exchange](#exchange)) expects asset index and not an asset id.

```javascript
api.tx.curveAmm.removeLiquidityOneCoin(poolId, lpAssetAmount, assetIndex, minAssetAmount)
```
