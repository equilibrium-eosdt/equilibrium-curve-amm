# Curve Amm Example Client
This program connects to the node,
sets up asset pool and performs adding,
exchanging and removing liquidity.

## Prepare and Run

Node.js should be already installed.

Example client uses yarn as a package manager. To install yarn type:

```shell
npm install --global yarn
```

Change directory to the `client` and install dependencies:

```shell
yarn install
```

Example client assumes that you run clean local node.
To clean state of the local dev node please type:

```shell
yes | cargo run -p node -- purge-chain --dev
```

Run node as usual:

```shell
cargo run -p node -- --dev
```

In the second terminal run client (from the `client` directory):

```shell
node index.js
```

## Client API

Here we describe all extrinsics that `equilibrium-curve-amm` pallet provides.
Example client calls every extrinsic. See [index.js](index.js) for the full code.

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
