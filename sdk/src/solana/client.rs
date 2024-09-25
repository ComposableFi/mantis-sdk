use crate::solana::{ChainError, Cluster};
use crate::Chain;
use anchor_client::Cluster as SolanaCluster;
use anchor_lang::prelude::Pubkey;
use async_trait::async_trait;
use mantis_common::UserIntent;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::{Keypair, Signer};
use std::sync::Arc;

pub(crate) struct SolanaClient {
    rpc_client: RpcClient,
    sender_keypair: Arc<Keypair>,
    #[allow(unused)]
    network: Cluster,
}

impl SolanaClient {
    pub fn new(network: Cluster, sender_keypair: Arc<Keypair>) -> Self {
        let (rpc_url, ws_url) = network.default_rpc_urls();
        Self::new_with_url(network, sender_keypair, rpc_url, ws_url)
    }

    pub fn new_with_url(
        network: Cluster,
        sender_keypair: Arc<Keypair>,
        rpc_url: &str,
        _ws_url: &str,
    ) -> Self {
        let rpc_client = RpcClient::new(rpc_url.to_string());

        Self {
            rpc_client,
            sender_keypair,
            network,
        }
    }
}

#[async_trait]
impl Chain for SolanaClient {
    type Transaction = ();
    type Address = Pubkey;
    type Token = ();
    type Amount = ();
    type Error = ChainError;

    async fn get_transaction(&self, _tx_hash: &str) -> Result<Self::Transaction, Self::Error> {
        unimplemented!("get_transaction not implemented for SolanaClient")
    }

    async fn submit_intent(
        &self,
        intent: UserIntent,
        program_id: Self::Address,
    ) -> Result<(), Self::Error> {
        let auctioneer = self.sender_keypair.clone();
        let _auctioneer_state = Pubkey::find_program_address(&[b"auctioneer"], &program_id).0;

        let client = anchor_client::Client::new_with_options(
            SolanaCluster::Custom(
                self.rpc_client.url().to_string(),
                self.rpc_client.url().to_string(),
            ),
            auctioneer.clone(),
            CommitmentConfig::processed(),
        );

        let program = client.program(program_id).map_err(|e| {
            ChainError::StoreIntentError(format!("Failed to get program instance: {}", e))
        })?;

        let intent_id = b"123"; // TODO
        let _intent_state = Pubkey::find_program_address(&[b"intent", intent_id], &program.id()).0;

        let result = program
            .request()
            .accounts(mantis_escrow_program::accounts::Initialize {})
            .args(mantis_escrow_program::instruction::EscrowAndStoreIntent {
                _amount: 0,
                _new_intent: intent,
            })
            .payer(auctioneer.clone())
            .signer(auctioneer.as_ref())
            .send_with_spinner_and_config(RpcSendTransactionConfig {
                skip_preflight: true,
                ..Default::default()
            });

        result
            .map_err(|e| ChainError::StoreIntentError(format!("Failed to send transaction: {}", e)))
            .map(|_| ())
    }

    fn signer(&self) -> Self::Address {
        self.sender_keypair.pubkey()
    }
}
