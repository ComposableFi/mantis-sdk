#![allow(unused)]

use std::sync::Arc;

use alloy::network::EthereumWallet;
use alloy::primitives::{address, Address, U256};
use alloy::providers::ProviderBuilder;
use alloy::rpc::types::TransactionReceipt;
use alloy::signers::local::{LocalSigner, PrivateKeySigner};
use alloy::sol;
use anyhow::Result;
use chrono::Utc;
use num::BigUint;
use rand::Rng;
use Escrow::NewIntent;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Escrow,
    "abis/escrow.json"
);

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    ERC20,
    "abis/erc20.json"
);

pub const ETH_TOKEN_ADDRESS: Address = address!("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE");
pub const INTENT_CHAIN_ID: u8 = 1;
pub const ETHEREUM_URL: &str = "https://rpc.flashbots.net/";
pub const ESCROW_SC_ADDRESS: Address = address!("0xAF55771e9cd32F93532670Ef358c8703d598505C");

async fn escrow_funds(
    signer: PrivateKeySigner,
    token_in: Address,
    amount_in: BigUint,
    token_out: String,
    amount_out: BigUint,
    dst_user: String,
    dst_chain_id: u8,
) -> Result<TransactionReceipt> {
    let wallet = EthereumWallet::from(signer.clone());
    let provider = Arc::new(
        ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(ETHEREUM_URL.parse()?),
    );
    let contract = Escrow::new(ESCROW_SC_ADDRESS, provider.clone());

    let intent_id = random_intent_id();

    let token_in_contract = ERC20::new(token_in, provider.clone());

    let amount_in = U256::from_le_slice(&amount_in.to_bytes_le());

    let tx_value = if token_in == ETH_TOKEN_ADDRESS {
        amount_in
    } else {
        U256::from(0)
    };

    let _receipt = token_in_contract
        .approve(ESCROW_SC_ADDRESS, amount_in)
        .from(signer.address())
        .send()
        .await?
        .get_receipt()
        .await?;

    let intent = NewIntent {
        intentId: intent_id,
        dstChainId: dst_chain_id,
        srcUser: signer.address(),
        dstUser: dst_user,
        tokenIn: token_in,
        amountIn: amount_in,
        tokenOut: token_out,
        amountOut: U256::from_le_slice(&amount_out.to_bytes_le()),
        timeout: U256::from(Utc::now().timestamp() + 360),
        aiAgent: false,
    };

    let receipt = contract
        .escrowFunds(intent)
        .from(signer.address())
        .value(tx_value)
        .send()
        .await?
        .get_receipt()
        .await?;

    Ok(receipt)
}

pub fn random_intent_id() -> u64 {
    rand::thread_rng().gen_range(100_000_000_000..=999_999_999_999)
}
