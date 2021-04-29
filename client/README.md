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
