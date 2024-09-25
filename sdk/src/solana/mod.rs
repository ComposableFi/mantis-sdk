use anchor_lang::prelude::thiserror::Error;
use anchor_lang::solana_program::pubkey::ParsePubkeyError;
use clap::ValueEnum;

pub(crate) mod cli;
pub(crate) mod client;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum Cluster {
    SolanaMainnet,
    SolanaTestnet,
    MantisMainnet,
    MantisTestnet,
}

impl Cluster {
    /// Returns a tuple of HTTP and Websocket RPC URLs for the cluster.
    fn default_rpc_urls(&self) -> (&str, &str) {
        match self {
            Cluster::SolanaMainnet => (
                "https://api.mainnet.solana.com",
                "wss://api.mainnet.solana.com",
            ),
            Cluster::SolanaTestnet => (
                "https://api.testnet.solana.com",
                "wss://api.testnet.solana.com",
            ),
            Cluster::MantisMainnet => (
                "https://mantis-rollup.composable-shared-artifacts.composablenodes.tech/rpc",
                "wss://mantis-rollup.composable-shared-artifacts.composablenodes.tech/rpc",
            ),
            Cluster::MantisTestnet => (
                "https://mantis-testnet.composable-shared-artifacts.composablenodes.tech/rpc",
                "wss://mantis-testnet.composable-shared-artifacts.composablenodes.tech/rpc",
            ),
        }
    }
}

#[allow(unused)]
#[derive(Error, Debug)]
pub enum ChainError {
    #[error("Failed to get transaction info: {0}")]
    TransactionInfoError(String),
    #[error("Transaction processing failed: {0}")]
    TransactionProcessingError(String),
    #[error("Transfer failed: {0}")]
    TransferError(String),
    #[error("Token account creation failed: {0}")]
    TokenAccountCreationError(String),
    #[error("Block number update failed: {0}")]
    BlockNumberError(String),
    #[error("Store intent failed: {0}")]
    StoreIntentError(String),
    #[error("Receive transfer failed: {0}")]
    ReceiveTransferError(String),
    #[error("Environment variable error: {0}")]
    EnvVarError(String),
    #[error("Invalid public key: {0}")]
    InvalidPubkey(String),
    #[error("Failed to parse Pubkey: {0}")]
    ParsePubkeyError(#[from] ParsePubkeyError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Other error: {0}")]
    Other(String),
}
