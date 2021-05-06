# Equilibrium Curve AMM Pallet

## Overview

Substrate-based runtime version of Curve automated market maker is an ongoing project which will deliver a following functionality:

- Low slippage, high efficiency stablecoin exchange

- High efficiency exchange tool for other homogeneous assets on Polkadot (e.g. wrapped assets) 

- Low risk fee income for liquidity providers. 

- Liquidity superfluidity with additional rewards from supplying liquidity to lending protocols such as Equilibrium and Acala.

Curve AMM is of paramount importance to the entire Polkadot ecosystem. With the introduction of parachains and interconnection of different Polka-based projects the issue of multiple wrapped assets representing the same underlying assets arises.

Consider ETH, for example. There are multiple bridging solutions who promise to introduce wrapped-ETH and other ERC-20 tokens to Polkadot. There needs to be a way to manage or exchange all of these representations of the same underlying asset inside Polkadot with low cost and low slippage, and that is where the Curve AMM comes into play.

Curve’s unique stableswap invariant utilizes liquidity much more efficiently compared to all existing DEXes for stablecoins at already several hundred USD TVL (total value locked). Since initial liquidity on Polkadot is hardly going to be very large, proposed efficiency is VERY important for the ecosystem to flourish.

## Installation

Make sure you have done all steps described in [Installation page](https://substrate.dev/docs/en/knowledgebase/getting-started/) of the Substrate Developer Hub.

To build project run:

```bash
cargo build
```

## Tests

To run unit tests type:

```bash
cargo test
```


In case you want run code coverage tool, please follow [instructions](https://github.com/xd009642/tarpaulin#installation) to install tarpaulin.

To create code coverage report run:

```bash
cargo tarpaulin -v
```

You can see code coverage report [here](reports/tarpaulin-report.html).

## Running the Node

First of all please ensure that your development chain's state is empty:

```bash
cargo run --bin node purge-chain --dev
```

Now you can start the development chain:

```bash
cargo run --bin node --dev
```

## Connecting to the Node

### Polkadot.js Explorer

It can be very useful to connect UI to the node you just started.

To do this open https://polkadot.js.org/apps/#/explorer in your browser first.

Follow these steps to register required custom types:

* In the main menu choose Settings tab;
* In the Settings submenu choose Developer tab;
* Copy content of the [custom-types.json](custom-types.json) file into text box on the page;
* Press Save button.

### Example Client

Example client connects to the clean dev node and performs various operations with `equilibrium-curve-amm` pallet.
See [this readme](client/README.md) for details.

## Using the Pallet

- See [Integration Guide](docs/INTEGRATION.md) for information on how to integrate `equilibrium-curve-amm` pallet into your chain.
- See [Client API](client/README.md#client-api) for how to use the pallet from the client perspective.

## Development Roadmap

| Milestone # | Description |
| --- | --- |
| 1 | Initial implementation, pool manipulations, invariant calculation |
| 2 | Actual assets exchange |

## License

Equilibrium Curve AMM pallet is [Apache 2.0 licensed](LICENSE).
