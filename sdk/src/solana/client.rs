use crate::solana::{ChainError, Cluster};
use crate::Chain;
use anchor_client::Cluster as SolanaCluster;
use anchor_lang::prelude::Pubkey;
use anchor_spl::associated_token::get_associated_token_address;
use async_trait::async_trait;
use mantis_common::UserIntent;
use rand::{distributions::Alphanumeric, Rng};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::{Keypair, Signer};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

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

        let intent_id: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(6)
            .map(char::from)
            .collect();
        let intent_state =
            Pubkey::find_program_address(&[b"intent", intent_id.as_bytes()], &program.id()).0;

        let current_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let new_intent = bridge_escrow::IntentPayload {
            intent_id: intent_id.clone(),
            user_in: auctioneer.pubkey(), // Must match the ctx.accounts.user key in the contract
            user_out: auctioneer.pubkey().to_string(),
            token_in: intent.token_in.parse()?,
            amount_in: intent
                .amount_in
                .parse()
                .map_err(|e: std::num::ParseIntError| ChainError::Other(e.to_string()))?,
            token_out: intent.token_out.clone(),
            amount_out: intent.amount_out.to_string(), // Amount out as a string
            timeout_timestamp_in_sec: current_timestamp + 10000, // Arbitrary timeout
            single_domain: true,
        };

        let auctioneer_state = Pubkey::find_program_address(&[b"auctioneer"], &bridge_escrow::ID).0;
        let user_token_account =
            get_associated_token_address(&auctioneer.pubkey(), &new_intent.token_in);
        let escrow_token_account =
            get_associated_token_address(&auctioneer_state, &new_intent.token_in);

        let result = program
            .request()
            .accounts(bridge_escrow::accounts::EscrowAndStoreIntent {
                user: auctioneer.pubkey(),
                user_token_account,
                auctioneer_state,
                token_mint: new_intent.token_in,
                escrow_token_account,
                intent: intent_state,
                token_program: anchor_spl::token::ID,
                associated_token_program: anchor_spl::associated_token::ID,
                system_program: anchor_lang::system_program::ID,
            })
            .args(bridge_escrow::instruction::EscrowAndStoreIntent {
                amount: new_intent.amount_in,
                new_intent,
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
