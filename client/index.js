const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const fs = require('fs');
const BigNumber = require('bignumber.js');

async function main() {
    const provider = new WsProvider('ws://localhost:9944');
    const types = JSON.parse(fs.readFileSync('../custom-types.json', 'utf8'));
    const api = await ApiPromise.create({ provider, types });

    const keyring = new Keyring({type: 'sr25519'});

    const alice = keyring.addFromUri('//Alice');

    // Create assets that will form a new pool
    const assetA = 0;
    const assetB = 1;
    const assetC = 2;
    const assetIds = [assetA, assetB, assetC];
    for (let asset of assetIds) {
        console.info(`Creating asset ${asset}...`)
        await includedInBlock(alice, api.tx.assets.create(asset, alice.address, 1, toBalance(5e-7)));
        console.info(`Asset ${asset} created.`);
    }

    // Mint assets to deposit into the pool
    const assetAmount = 1000;
    for (let asset of assetIds) {
        console.info(`Minting asset ${asset}...`)
        await includedInBlock(alice, api.tx.assets.mint(asset, alice.address, toBalance(10 * assetAmount)));
        console.info(`Asset ${asset} minted.`);
    }

    // Create the pool and put all assets created previously into it
    console.info('Creating pool...');
    const amplification = toFixedNumber(1);
    const fee = 0;
    const adminFee = 0;
    const poolId = getPoolId(await includedInBlock(alice, api.tx.curveAmm.createPool(assetIds, amplification, fee, adminFee)));
    console.info(`Pool ${poolId.toHuman()} created.`);

    // Detect asset id of lp asset of the created pool
    const poolInfo = await api.query.curveAmm.pools(poolId);
    const assetLp = poolInfo.unwrap().pool_asset;
    console.info(`Lp asset for pool ${poolId.toHuman()} is ${assetLp.toHuman()}`);

    // Initial liquidity must be deposited for all assets in the pool
    console.info('Depositing initial liquidity to the pool...');
    {
        const assetAmounts = assetIds.map(_ => toBalance(assetAmount));
        const minMintAmount = toBalance(0);
        await includedInBlock(alice, api.tx.curveAmm.addLiquidity(poolId, assetAmounts, minMintAmount));
    }
    console.info('Initial liquidity deposited.')

    // All subsequent liquidity deposits can be for some subset of assets in the pool
    console.info('Depositing liquidity only for first two assets to the pool...');
    {
        const assetAmounts = assetIds.map((_, i) => toBalance(i < 2 ? assetAmount : 0));
        const minMintAmount = toBalance(0);
        await includedInBlock(alice, api.tx.curveAmm.addLiquidity(poolId, assetAmounts, minMintAmount));
    }
    console.info('Liquidity deposited.');

    // Exchange 10 units of assetC for assetA
    console.info('Exchanging assets...')
    {
        const fromAssetIndex = assetIds.indexOf(assetC);
        const toAssetIndex = assetIds.indexOf(assetA);
        const fromAssetAmount = toBalance(10);
        const toAssetMinAmount = toBalance(0);
        await includedInBlock(alice, api.tx.curveAmm.exchange(poolId, fromAssetIndex, toAssetIndex, fromAssetAmount, toAssetMinAmount));
    }
    console.info('Assets exchanged.');

    // We can remove liquidity in a balanced way
    console.info('Removing liquidity...')
    {
        const lpAssetAmount = toBalance(3);
        const minAssetAmounts = assetIds.map(_ => toBalance(0));
        await includedInBlock(alice, api.tx.curveAmm.removeLiquidity(poolId, lpAssetAmount, minAssetAmounts));
    }
    console.info('Liquidity removed.');

    // But also we can remove liquidity with more freedom
    console.info('Removing liquidity in imbalanced way...')
    {
        const assetAmounts = assetIds.map((x, i) => toBalance(i < 2 ? 1 : 0));
        const lpMaxBurnAmount = toBalance(assetIds.length);
        await includedInBlock(alice, api.tx.curveAmm.removeLiquidityImbalance(poolId, assetAmounts, lpMaxBurnAmount));
    }
    console.info('Liquidity removed imbalanced.');

    // Also we can remove liquidity for single asset
    console.info('Removing liquidity for assetB...')
    {
        const lpAssetAmount = toBalance(3);
        const assetIndex = assetIds.indexOf(assetB);
        const minAssetAmount = toBalance(0);
        await includedInBlock(alice, api.tx.curveAmm.removeLiquidityOneCoin(poolId, lpAssetAmount, assetIndex, minAssetAmount));
    }
    console.info('Liquidity for assetB removed.');
}

function includedInBlock(signer, txCall) {
    return new Promise((resolve, reject) => {
        let unsub = null;
        txCall.signAndSend(signer, (result) => {
            if (result.status.isInBlock) {
                if (unsub == null) {
                    reject('Unsub still not available');
                } else {
                    unsub();
                    resolve(result.events);
                }
            }
        }).then(x => {unsub = x;}, err => reject(err));
    });
}

function getPoolId(events) {
    return events[1].event.data[1];
}

function toFixedNumber(num) {
    return new BigNumber(num).multipliedBy(new BigNumber(1e18)).toString();
}

function toBalance(num) {
    return new BigNumber(num).multipliedBy(new BigNumber(1e9)).toString();
}

(async () => {
    main().catch(e => {
        console.error(`Something went horribly wrong: ${e.message}`);
    });
})();