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
    const assets = [assetA, assetB, assetC];
    for (let asset of assets) {
        console.info(`Creating asset ${asset}...`)
        await includedInBlock(alice, api.tx.assets.create(asset, alice.address, 1, toBalance(5e-7)));
        console.info(`Asset ${asset} created.`);
    }

    // Mint assets to deposit into the pool
    const assetAmount = 1000;
    for (let asset of assets) {
        console.info(`Minting asset ${asset}...`)
        await includedInBlock(alice, api.tx.assets.mint(asset, alice.address, toBalance(10 * assetAmount)));
        console.info(`Asset ${asset} minted.`);
    }

    // Create the pool and put all assets created previously into it
    console.info('Creating pool...');
    const poolId = getPoolId(await includedInBlock(alice, api.tx.curveAmm.createPool(assets, toFixedNumber(1), 0, 0)));
    console.info(`Pool ${poolId.toHuman()} created.`);

    // Detect asset id of lp asset of the created pool
    const poolInfo = await api.query.curveAmm.pools(poolId);
    const assetLp = poolInfo.unwrap().pool_asset;
    console.info(`Lp asset for pool ${poolId.toHuman()} is ${assetLp.toHuman()}`);

    // Initial liquidity must be deposited for all assets in the pool
    console.info('Depositing initial liquidity to the pool...');
    await includedInBlock(alice, api.tx.curveAmm.addLiquidity(poolId, assets.map(_ => toBalance(assetAmount)), toBalance(0)));
    console.info('Initial liquidity deposited.')

    // All subsequent liquidity deposits can be for some subset of assets in the pool
    console.info('Depositing liquidity only for first two assets to the pool...');
    await includedInBlock(alice, api.tx.curveAmm.addLiquidity(
        poolId,
        assets.map((_, i) => toBalance(i < 2 ? assetAmount : 0)), toBalance(0))
    );
    console.info('Liquidity deposited.');

    // Exchange 10 units of assetC for assetA
    console.info('Exchanging assets...')
    await includedInBlock(alice, api.tx.curveAmm.exchange(poolId, assetC, assetA, toBalance(10), toBalance(0)));
    console.info('Assets exchanged.');

    // We can remove liquidity in a balanced way
    console.info('Removing liquidity...')
    await includedInBlock(alice, api.tx.curveAmm.removeLiquidity(poolId, toBalance(3), assets.map(_ => toBalance(0))));
    console.info('Liquidity removed.');

    // But also we can remove liquidity with more freedom
    console.info('Removing liquidity in imbalanced way...')
    await includedInBlock(alice, api.tx.curveAmm.removeLiquidityImbalance(poolId, assets.map((x, i) => toBalance(i < 2 ? 1 : 0)), toBalance(assets.length)));
    console.info('Liquidity removed imbalanced.');

    // Also we can remove liquidity for single asset
    console.info('Removing liquidity for assetB...')
    await includedInBlock(alice, api.tx.curveAmm.removeLiquidityOneCoin(poolId, toBalance(3), assetB, toBalance(0)));
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