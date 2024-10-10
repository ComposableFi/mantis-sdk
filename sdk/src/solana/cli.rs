use crate::credentials::Credential;
use crate::solana::client::SolanaClient;
use crate::solana::{ChainError, Cluster};
use clap::Args;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::SeedDerivable;
use std::sync::Arc;

// TODO: improve keypair handling (load from raw keys, files, interactive, etc.)
#[derive(Args)]
pub(crate) struct SolanaArgs {
    /// Mnemonic seed phrase
    #[arg(long, short, env = "SOLANA_MNEMONIC", conflicts_with = "keypair")]
    pub(crate) mnemonic: Option<String>,
    /// Filepath or URL to a keypair
    #[arg(long, short, env = "SOLANA_KEYPAIR", conflicts_with = "mnemonic")]
    pub(crate) keypair: Option<String>,
    #[arg(long, env = "SOLANA_RPC_URL", requires = "ws_url")]
    pub(crate) rpc_url: Option<String>,
    #[arg(long, env = "SOLANA_WS_URL", requires = "rpc_url")]
    pub(crate) ws_url: Option<String>,
    #[arg(long, env = "SOLANA_CLUSTER")]
    pub(crate) cluster: Cluster,
}

impl SolanaArgs {
    pub(crate) async fn build_client(&self) -> Result<SolanaClient, ChainError> {
        let keypair = Arc::new(self.build_signer()?);

        let client = if self.rpc_url.is_none() {
            // if RPCs are not provided, use default ones depending on the cluster
            SolanaClient::new(self.cluster, keypair.clone())
        } else {
            let (rpc_url, ws_url) = (
                self.rpc_url.as_ref().unwrap(),
                self.ws_url.as_ref().unwrap(),
            );
            SolanaClient::new_with_url(self.cluster, keypair.clone(), rpc_url, ws_url)
        };
        Ok(client)
    }

    pub fn build_signer(&self) -> Result<Keypair, ChainError> {
        match Credential::from_options(&self.keypair, &self.mnemonic) {
            None => Err(ChainError::Other(
                "Exactly one of keypair or mnemonic must be provided".to_string(),
            )),
            Some(Credential::Keypair(path)) => {
                let file_contents = std::fs::read_to_string(&path)?;
                Ok(Keypair::from_base58_string(&file_contents))
            }
            Some(Credential::Mnemonic(mnemonic)) => {
                Keypair::from_seed_phrase_and_passphrase(&mnemonic, "")
                    .map_err(|e| ChainError::Other(e.to_string()))
            }
        }
    }
}
