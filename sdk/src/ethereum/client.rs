use crate::ethereum::{ChainError, Network};
use crate::Chain;
use alloy::primitives::{Address, U256};
use alloy::signers::local::PrivateKeySigner;
use alloy::rpc::types::Transaction;
use async_trait::async_trait;
use mantis_common::UserIntent;
use std::sync::Arc;
use std::time::Duration;
use alloy::providers::{ProviderBuilder, ReqwestProvider};
use alloy::sol;
use reqwest::Url;
use crate::ethereum::client::Escrow::{EscrowInstance, IntentInfo};

sol!(
    #[sol(rpc)]
    Escrow,
    "../contracts/ethereum/abi/escrow.json"
);

type RpcProvider = ReqwestProvider;

pub(crate) struct EthereumClient {
    #[allow(unused)]
    rpc_client: RpcProvider,
    sender_keypair: Arc<PrivateKeySigner>,
    #[allow(unused)]
    network: Network,
}

impl EthereumClient {
    pub fn new(network: Network, sender_keypair: Arc<PrivateKeySigner>) -> Self {
        let (rpc_url, ws_url) = network.default_rpc_urls();
        Self::new_with_url(network, sender_keypair, rpc_url, ws_url)
    }

    pub fn new_with_url(
        network: Network,
        sender_keypair: Arc<PrivateKeySigner>,
        rpc_url: Url,
        _ws_url: Url,
    ) -> Self {
        let provider = ProviderBuilder::new().on_http(rpc_url);
        // TODO: specify escrow address

        Self {
            rpc_client: provider,
            sender_keypair,
            network,
        }
    }
}

#[async_trait]
impl Chain for EthereumClient {
    type Transaction = Transaction;
    type Address = Address;
    type Token = String;
    type Amount = U256;
    type Error = ChainError;

    async fn get_transaction(&self, _tx_hash: &str) -> Result<Self::Transaction, Self::Error> {
        todo!()
    }

    async fn submit_intent(
        &self,
        intent: UserIntent,
        _contract_address: Self::Address,
    ) -> Result<(), Self::Error> {
        let intent_id = "123".to_string(); // TODO
        let escrow = EscrowInstance::new(Address::default(), self.rpc_client.clone());
        let method = escrow.escrowFunds(
            intent_id,
            IntentInfo {
                tokenIn: intent.token_in.parse().map_err(|_| ChainError::ParseAddressError)?,
                amountIn: intent.amount_in.parse()?,
                srcUser: intent.user_address.parse().map_err(|_| ChainError::ParseAddressError)?,
                tokenOut: intent.token_out,
                amountOut: intent.amount_out,
                dstUser: "".to_string(),
                winnerSolver: "".to_string(),
                timeout: Default::default(),
            }
        );
        let _tx_hash = method.send().await?.with_timeout(Some(Duration::from_secs(30))).watch().await?;
        Ok(())
    }

    fn signer(&self) -> Self::Address {
        self.sender_keypair.address()
    }
}
