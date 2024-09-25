use std::sync::Arc;
use alloy::signers::k256::ecdsa;
use alloy::signers::local::coins_bip39::{English, Mnemonic};
use alloy::signers::local::PrivateKeySigner;
use super::{ChainError, Network};
use clap::Args;
use reqwest::Url;
use crate::ethereum::client::EthereumClient;

// TODO: improve keypair handling (load from raw keys, files, interactive, etc.)
#[derive(Args)]
pub(crate) struct EthereumArgs {
    /// Mnemonic seed phrase
    #[arg(long, short, env = "ETHEREUM_MNEMONIC", conflicts_with = "keypair")]
    pub(crate) mnemonic: String,
    /// Filepath or URL to a keypair
    #[arg(long, short, env = "ETHEREUM_KEYPAIR", conflicts_with = "mnemonic")]
    pub(crate) keypair: String,
    #[arg(long, env = "ETHEREUM_RPC_URL", requires = "ws_url")]
    pub(crate) rpc_url: Option<Url>,
    #[arg(long, env = "ETHEREUM_WS_URL", requires = "rpc_url")]
    pub(crate) ws_url: Option<Url>,
    #[arg(long, env = "ETHEREUM_CLUSTER")]
    pub(crate) network: Network,
}

impl EthereumArgs {
    pub(crate) async fn build_client(&self) -> Result<EthereumClient, ChainError> {
        let keypair = Arc::new(self.build_signer()?);

        let client = if self.rpc_url.is_none() {
            // if RPCs are not provided, use default ones depending on the cluster
            EthereumClient::new(self.network, keypair)
        } else {
            let (rpc_url, ws_url) = (
                self.rpc_url.clone().unwrap(),
                self.ws_url.clone().unwrap(),
            );
            EthereumClient::new_with_url(
                self.network,
                keypair,
                rpc_url,
                ws_url,
            )
        };
        Ok(client)
    }

    pub fn build_signer(&self) -> Result<PrivateKeySigner, ChainError> {
        if !self.mnemonic.is_empty() {
            let x_priv = Mnemonic::<English>::new_from_phrase(&self.mnemonic)?.master_key(None)?;
            let x: &ecdsa::SigningKey = x_priv.as_ref();
            Ok(PrivateKeySigner::from_signing_key(x.clone()))
        } else {
            PrivateKeySigner::decrypt_keystore(&self.keypair, "").map_err(Into::into)
        }
    }
}