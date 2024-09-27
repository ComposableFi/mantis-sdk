# Mantis SDK

Mantis SDK is a command-line interface (CLI) and library for interacting with the Mantis L2 rollup. It allows users to submit intents for token swaps and retrieve quotes, as well as providing a Rust API for developers to integrate Mantis functionalities into their applications.

## Table of Contents

- [Installation](#installation)
    - [As a CLI](#as-a-cli)
    - [As a Dependency](#as-a-dependency)
- [CLI Usage](#cli-usage)
    - [Commands](#commands)
        - [`intent submit solana`](#intent-submit-solana)
        - [`intent submit ethereum`](#intent-submit-ethereum)
        - [`get-quote`](#get-quote)
- [Library Usage](#library-usage)
    - [Adding to Your Project](#adding-to-your-project)
    - [Using `SolanaClient`](#using-solanaclient)
    - [Using `EthereumClient`](#using-ethereumclient)
- [Traits and Enums](#traits-and-enums)
    - [`Chain` Trait](#chain-trait)
    - [`Cluster` Enum](#cluster-enum)
    - [`Network` Enum](#network-enum)
- [Examples](#examples)

## Installation

### As CLI

To install the Mantis SDK CLI, ensure you have Rust and Cargo installed. Then, build the project:

```bash
git clone https://github.com/ComposableFi/mantis-sdk.git
cd mantis-sdk
cargo install --path .
```

This will compile the `mantis-sdk` binary and install it to your Cargo bin directory.

### As a dependency

To use Mantis SDK as a library in your Rust project, add it to your `Cargo.toml`:

```toml
[dependencies]
mantis-sdk = "0.1.0"
```

## CLI Usage

The `mantis-sdk` CLI allows you to submit intents and get quotes for token exchanges on Solana and Ethereum networks.

### Commands

#### `intent submit solana`

Submit an intent to exchange tokens on the Solana network.

```bash
mantis-sdk intent submit solana [OPTIONS] <TOKEN_IN_NAME> <AMOUNT_IN> <TOKEN_OUT_NAME> <AMOUNT_OUT>
```

**Arguments:**

- `<TOKEN_IN_NAME>`: The name of the token you want to exchange.
- `<AMOUNT_IN>`: The amount of the input token.
- `<TOKEN_OUT_NAME>`: The name of the token you want to receive.
- `<AMOUNT_OUT>`: The desired amount of the output token.

**Options:**

- `-m, --mnemonic <MNEMONIC>`: Mnemonic seed phrase (conflicts with `--keypair`). [env: `SOLANA_MNEMONIC`]
- `-k, --keypair <KEYPAIR>`: Filepath or URL to a keypair (conflicts with `--mnemonic`). [env: `SOLANA_KEYPAIR`]
- `--rpc-url <RPC_URL>`: Custom RPC URL. [env: `SOLANA_RPC_URL`]
- `--ws-url <WS_URL>`: Custom WebSocket URL. [env: `SOLANA_WS_URL`]
- `--cluster <CLUSTER>`: Solana cluster to connect to (`solana-mainnet`, `solana-testnet`, `mantis-mainnet`, `mantis-testnet`). [env: `SOLANA_CLUSTER`]

**Example:**

```bash
mantis-sdk intent submit solana \
  --keypair /path/to/keypair.json \
  --cluster solana-mainnet \
  SOL 10 ETH 20
```

#### `intent submit ethereum`

Submit an intent to exchange tokens on the Ethereum network.

```bash
mantis-sdk intent submit ethereum [OPTIONS] <TOKEN_IN_NAME> <AMOUNT_IN> <TOKEN_OUT_NAME> <AMOUNT_OUT>
```

**Arguments:**

- Same as for `solana`.

**Options:**

- `-m, --mnemonic <MNEMONIC>`: Mnemonic seed phrase (conflicts with `--keypair`). [env: `ETHEREUM_MNEMONIC`]
- `-k, --keypair <KEYPAIR>`: Filepath or URL to a keypair (conflicts with `--mnemonic`). [env: `ETHEREUM_KEYPAIR`]
- `--rpc-url <RPC_URL>`: Custom RPC URL. [env: `ETHEREUM_RPC_URL`]
- `--ws-url <WS_URL>`: Custom WebSocket URL. [env: `ETHEREUM_WS_URL`]
- `--network <NETWORK>`: Ethereum network to connect to (`ethereum-mainnet`, `ethereum-sepolia`). [env: `ETHEREUM_NETWORK`]

**Example:**

```bash
mantis-sdk intent submit ethereum \
  --mnemonic "your mnemonic phrase" \
  --network ethereum-mainnet \
  DAI 100 USDC 100
```

#### `get-quote`

Retrieve a quote for a token swap.

```bash
mantis-sdk get-quote <TOKEN_IN_NAME> <TOKEN_OUT_NAME> <AMOUNT_IN>
```

**Arguments:**

- `<TOKEN_IN_NAME>`: The name of the token you want to exchange.
- `<TOKEN_OUT_NAME>`: The name of the token you want to receive.
- `<AMOUNT_IN>`: The amount of the input token.

**Example:**

```bash
mantis-sdk get-quote BTC ETH 1
```

### Help

To display help information for any command or subcommand:

```bash
mantis-sdk --help
mantis-sdk intent submit solana --help
```

## Library Usage

### Adding to Your Project

Include `mantis-sdk` in your `Cargo.toml`:

```toml
[dependencies]
mantis-sdk = "0.1.0"
```

### Using `SolanaClient`

To interact with the Solana network:

```rust
use mantis_sdk::{solana::SolanaClient, solana::Cluster};
use std::sync::Arc;

fn main() {
    let cluster = Cluster::SolanaMainnet;
    let keypair = Arc::new(/* Load your keypair here */);

    // Without custom RPC URLs
    let client = SolanaClient::new(cluster, keypair);

    // With custom RPC and WS URLs
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let ws_url = "wss://api.mainnet-beta.solana.com";
    let client = SolanaClient::new_with_urls(cluster, keypair, rpc_url, ws_url);
}
```

### Using `EthereumClient`

To interact with the Ethereum network:

```rust
use mantis_sdk::{ethereum::EthereumClient, ethereum::Network};
use std::sync::Arc;

fn main() {
    let network = Network::EthereumMainnet;
    let keypair = Arc::new(/* Load your keypair here */);

    // Without custom RPC URLs
    let client = EthereumClient::new(network, keypair);

    // With custom RPC and WS URLs
    let rpc_url = "https://mainnet.infura.io/v3/your-project-id";
    let ws_url = "wss://mainnet.infura.io/ws/v3/your-project-id";
    let client = EthereumClient::new_with_urls(network, keypair, rpc_url, ws_url);
}
```

## Traits and Enums

### `Chain` Trait

Both `SolanaClient` and `EthereumClient` implement the `Chain` trait:

```rust
#[async_trait]
pub trait Chain {
    type Transaction;
    type Address;
    type Token;
    type Amount;
    type Error;

    async fn get_transaction(&self, tx_hash: &str) -> Result<Self::Transaction, Self::Error>;

    async fn submit_intent(
        &self,
        intent: UserIntent,
        address: Self::Address,
    ) -> Result<(), Self::Error>;

    fn signer(&self) -> Self::Address;
}
```

### `Cluster` Enum

Represents Solana clusters:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum Cluster {
    SolanaMainnet,
    SolanaTestnet,
    MantisMainnet,
    MantisTestnet,
}
```

### `Network` Enum

Represents Ethereum networks:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum Network {
    EthereumMainnet,
    EthereumSepolia,
}
```

## Examples

### Submitting an Intent on Solana

```bash
mantis-sdk intent submit solana \
  --keypair /path/to/keypair.json \
  --cluster solana-mainnet \
  SOL 10 ETH 20
```

### Submitting an Intent on Ethereum

```bash
mantis-sdk intent submit ethereum \
  --mnemonic "your mnemonic phrase" \
  --network ethereum-mainnet \
  DAI 100 USDC 100
```

### Getting a Quote

```bash
mantis-sdk get-quote BTC ETH 1
```

### Using the Library in Code

```rust
use mantis_sdk::solana::{SolanaClient, Cluster};
use mantis_sdk::ethereum::{EthereumClient, Network};
use mantis_sdk::Chain;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Solana Client Example
    let solana_cluster = Cluster::SolanaMainnet;
    let solana_keypair = Arc::new(/* Load your Solana keypair */);
    let solana_client = SolanaClient::new(solana_cluster, solana_keypair);

    // Ethereum Client Example
    let ethereum_network = Network::EthereumMainnet;
    let ethereum_keypair = Arc::new(/* Load your Ethereum keypair */);
    let ethereum_client = EthereumClient::new(ethereum_network, ethereum_keypair);

    // Example usage of Chain trait methods
    let tx_hash = "some_transaction_hash";

    // Solana transaction
    if let Ok(transaction) = solana_client.get_transaction(tx_hash).await {
        // Process transaction
    }

    // Ethereum transaction
    if let Ok(transaction) = ethereum_client.get_transaction(tx_hash).await {
        // Process transaction
    }
}
```

---

*Note: Replace placeholders like `/* Load your keypair */` with actual code to load your keypairs. Ensure that you handle private keys securely and never commit them to version control.*

## Additional Information

- **Environment Variables:** You can set environment variables to avoid passing sensitive information via command-line arguments.

  ```bash
  export SOLANA_MNEMONIC="your mnemonic phrase"
  export SOLANA_CLUSTER="solana-mainnet"
  export ETHEREUM_KEYPAIR="/path/to/keypair.json"
  export ETHEREUM_NETWORK="ethereum-mainnet"
  ```

- **Help Command:** Use `--help` with any command to display detailed usage information.

- **Version:** Check the version of the CLI using:

  ```bash
  mantis-sdk --version
  ```

- **Dependencies:** Ensure you have the necessary dependencies in your `Cargo.toml` when using the SDK as a library.

  ```toml
  [dependencies]
  mantis-sdk = "0.1.0"
  async-trait = "0.1.51"  # Required for the Chain trait
  ```

---

For more information, refer to the [documentation](#TODO) or open an issue on the [GitHub repository](https://github.com/ComposableFi/mantis-sdk/issues).