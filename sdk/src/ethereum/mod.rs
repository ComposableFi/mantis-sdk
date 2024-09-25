pub(crate) mod cli;
mod client;

use alloy::signers::local::coins_bip39::MnemonicError;
use alloy::signers::local::LocalSignerError;
use anchor_lang::prelude::thiserror::Error;
use anchor_lang::solana_program::pubkey::ParsePubkeyError;
use clap::ValueEnum;
use reqwest::Url;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum Network {
    EthereumMainnet,
    EthereumSepolia,
}

impl Network {
    /// Returns a tuple of HTTP and Websocket RPC URLs for the cluster.
    fn default_rpc_urls(&self) -> (Url, Url) {
        // TODO: not actually working
        let (rpc, ws) = match self {
            Network::EthereumMainnet => (
                "https://mainnet.infura.io/v3/8f2e4b3b3f4b4f3b8f2e4b3b3f4b4f3b",
                "wss://mainnet.infura.io/ws/v3/8f2e4b3b3f4b4f3b8f2e4b3b3f4b4f3b",
            ),
            Network::EthereumSepolia => (
                "https://sepolia.infura.io/v3/8f2e4b3b3f4b4f3b8f2e4b3b3f4b4f3b",
                "wss://sepolia.infura.io/ws/v3/8f2e4b3b3f4b4f3b8f2e4b3b3f4b4f3b",
            ),
        };
        (Url::parse(rpc).unwrap(), Url::parse(ws).unwrap())
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
    #[error("Error forming Mnemonic: {0}")]
    MnemonicError(#[from] MnemonicError),
    #[error("Local signer error: {0}")]
    LocalSignerError(#[from] LocalSignerError),
    #[error("Failed to parse ethereum Address")]
    ParseAddressError,
    #[error("Failed to parse uint: {0}")]
    ParseUintError(#[from] ruint::ParseError),
    #[error("Contract error: {0}")]
    ContractError(#[from] alloy::contract::Error),
    #[error("Pending transaction error: {0}")]
    PendingTransactionError(#[from] alloy::providers::PendingTransactionError),
    #[error("Other error: {0}")]
    Other(String),
}
