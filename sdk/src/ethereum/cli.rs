use super::{ChainError, Network};
use crate::credentials::Credential;
use crate::ethereum::client::EthereumClient;
use alloy::signers::k256::ecdsa;
use alloy::signers::local::coins_bip39::{English, Mnemonic};
use alloy::signers::local::PrivateKeySigner;
use clap::Args;
use reqwest::Url;
use std::sync::Arc;

// TODO: improve keypair handling (load from raw keys, files, interactive, etc.)
#[derive(Args)]
pub(crate) struct EthereumArgs {
    /// Mnemonic seed phrase
    #[arg(long, short, env = "ETHEREUM_MNEMONIC", conflicts_with = "keypair")]
    pub(crate) mnemonic: Option<String>,
    /// Filepath or URL to a keypair
    #[arg(long, short, env = "ETHEREUM_KEYPAIR", conflicts_with = "mnemonic")]
    pub(crate) keypair: Option<String>,
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
            let (rpc_url, ws_url) = (self.rpc_url.clone().unwrap(), self.ws_url.clone().unwrap());
            EthereumClient::new_with_url(self.network, keypair, rpc_url, ws_url)
        };
        Ok(client)
    }

    pub fn build_signer(&self) -> Result<PrivateKeySigner, ChainError> {
        match Credential::from_options(&self.keypair, &self.mnemonic) {
            None => Err(ChainError::Other(
                "Exactly one of keypair or mnemonic must be provided".to_string(),
            )),
            Some(Credential::Keypair(path)) => {
                PrivateKeySigner::decrypt_keystore(path, "").map_err(Into::into)
            }
            Some(Credential::Mnemonic(mnemonic)) => {
                let x_priv = Mnemonic::<English>::new_from_phrase(mnemonic)?.master_key(None)?;
                let x: &ecdsa::SigningKey = x_priv.as_ref();
                Ok(PrivateKeySigner::from_signing_key(x.clone()))
            }
        }
    }
}
