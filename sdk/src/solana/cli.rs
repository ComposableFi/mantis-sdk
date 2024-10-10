use crate::credentials::{self, Credential};
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
        let keypair: Arc<Keypair> =
            Arc::new(credentials::from_options(&self.keypair, &self.mnemonic)?);

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
}

impl Credential for Keypair {
    type Error = ChainError;

    fn from_base58_file(filename: &str) -> Result<Self, Self::Error> {
        let file_contents = std::fs::read_to_string(&filename)?;
        Ok(Keypair::from_base58_string(&file_contents))
    }

    fn from_mnemonic(mnemonic: &str) -> Result<Self, Self::Error> {
        Keypair::from_seed_phrase_and_passphrase(&mnemonic, "")
            .map_err(|e| ChainError::Other(e.to_string()))
    }

    fn error_from_str(s: &str) -> Self::Error {
        ChainError::Other(s.to_string())
    }
}
