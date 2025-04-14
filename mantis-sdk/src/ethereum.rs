use alloy::network::Ethereum;
use alloy::primitives::{address, Address, Bytes, TxKind, U256};
use alloy::providers::{Provider, WalletProvider};
use alloy::rpc::types::{TransactionInput, TransactionReceipt, TransactionRequest};
use alloy::sol;
use alloy::transports::Transport;
use anyhow::{anyhow, Result};
use chrono::Utc;
use tracing::{info, instrument};
use Escrow::NewIntent;

use crate::{random_intent_id, retry};

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

#[derive(Default, Debug, Clone)]
pub struct GasFees {
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
}

#[instrument(skip_all)]
pub async fn escrow_funds<P, T>(
    provider: P,
    escrow_address: Address,
    token_in: Address,
    amount_in: U256,
    token_out: String,
    amount_out: U256,
    dst_user: String,
    dst_chain_id: u8,
    timeout_sec: u64,
    ai_agent: bool,
) -> Result<TransactionReceipt>
where
    P: Provider<T, Ethereum> + Clone + WalletProvider<Ethereum>,
    T: Transport + Clone,
{
    let escrow_contract = Escrow::new(escrow_address, provider.clone());

    let intent_id = random_intent_id();

    let tx_value = (token_in == ETH_TOKEN_ADDRESS)
        .then_some(amount_in)
        .unwrap_or_default();

    let receipt = approve_erc20(provider.clone(), token_in, escrow_address, amount_in).await?;

    info!(
        "Approved {} of token {} to {} ({})",
        amount_in,
        token_in.to_checksum(None),
        escrow_address.to_checksum(None),
        receipt.transaction_hash,
    );

    let intent = NewIntent {
        intentId: intent_id,
        dstChainId: dst_chain_id,
        srcUser: provider.default_signer_address(),
        dstUser: dst_user,
        tokenIn: token_in,
        amountIn: amount_in,
        tokenOut: token_out,
        amountOut: amount_out,
        timeout: U256::from(Utc::now().timestamp() as u64 + timeout_sec),
        aiAgent: ai_agent,
    };

    let pending = retry(
        || async {
            escrow_contract
                .escrowFunds(intent.clone())
                .value(tx_value)
                .send()
                .await
        },
        3,
    )
    .await?;

    let tx_hash = pending.tx_hash();

    info!(
        "Ethereum escrowFunds transaction {} was sent to the network",
        tx_hash
    );

    let receipt = retry(
        || async {
            provider
                .get_transaction_receipt(*tx_hash)
                .await?
                .ok_or(anyhow!("No transaction receipt"))
        },
        10,
    )
    .await?;

    Ok(receipt)
}

#[instrument(skip_all)]
pub async fn solve_intent_remote<P, T>(
    provider: P,
    escrow_address: Address,
    intent_id: u64,
    token_out: Address,
    amount_out: U256,
    dst_user: Address,
    dst_chain_id: u8,
    tx_value: U256,
) -> Result<TransactionReceipt>
where
    P: Provider<T, Ethereum> + Clone + WalletProvider<Ethereum>,
    T: Transport + Clone,
{
    let escrow_contract = Escrow::new(escrow_address, provider.clone());

    let pending = retry(
        || async {
            escrow_contract
                .solveIntentRemote(intent_id, dst_chain_id, token_out, amount_out, dst_user)
                .value(tx_value)
                .send()
                .await
        },
        3,
    )
    .await?;

    let tx_hash = pending.tx_hash();

    info!(
        "Ethereum solveIntentRemote transaction {} was sent to the network",
        tx_hash
    );

    let receipt = retry(
        || async {
            provider
                .get_transaction_receipt(*tx_hash)
                .await?
                .ok_or(anyhow!("No transaction receipt"))
        },
        10,
    )
    .await?;

    Ok(receipt)
}

#[instrument(skip_all)]
pub async fn solve_intent_local<P, T>(
    provider: P,
    escrow_address: Address,
    intent_id: u64,
    tx_value: U256,
) -> Result<TransactionReceipt>
where
    P: Provider<T, Ethereum> + Clone + WalletProvider<Ethereum>,
    T: Transport + Clone,
{
    let escrow_contract = Escrow::new(escrow_address, provider.clone());

    let pending = retry(
        || async {
            escrow_contract
                .solveIntentLocal(intent_id)
                .value(tx_value)
                .send()
                .await
        },
        3,
    )
    .await?;

    let tx_hash = pending.tx_hash();

    info!(
        "Ethereum solveIntentLocal transaction {} was sent to the network",
        tx_hash
    );

    let receipt = retry(
        || async {
            provider
                .get_transaction_receipt(*tx_hash)
                .await?
                .ok_or(anyhow!("No transaction receipt"))
        },
        10,
    )
    .await?;

    Ok(receipt)
}

#[instrument(skip_all)]
pub async fn cancel_intent<P, T>(
    provider: P,
    escrow_address: Address,
    intent_id: u64,
) -> Result<TransactionReceipt>
where
    P: Provider<T, Ethereum> + Clone + WalletProvider<Ethereum>,
    T: Transport + Clone,
{
    let contract = Escrow::new(escrow_address, provider.clone());

    let pending = retry(|| async { contract.cancelIntent(intent_id).send().await }, 3).await?;

    let tx_hash = pending.tx_hash();

    info!(
        "Ethereum cancelIntent transaction {} was sent to the network",
        tx_hash
    );

    let receipt = retry(
        || async {
            provider
                .get_transaction_receipt(*tx_hash)
                .await?
                .ok_or(anyhow!("No transaction receipt"))
        },
        10,
    )
    .await?;

    info!("Canceled intent {} on Ethereum", intent_id);
    Ok(receipt)
}

#[instrument(skip_all)]
pub async fn approve_erc20<P, T>(
    provider: P,
    token: Address,
    spender: Address,
    amount: U256,
) -> Result<TransactionReceipt>
where
    P: Provider<T, Ethereum> + Clone + WalletProvider<Ethereum>,
    T: Transport + Clone,
{
    let token_contract = ERC20::new(token, provider.clone());

    let pending = retry(
        || async { token_contract.approve(spender, amount).send().await },
        3,
    )
    .await?;

    let tx_hash = pending.tx_hash();

    info!("Ethereum approve transaction {} was sent to the network", tx_hash);

    let receipt = retry(
        || async {
            provider
                .get_transaction_receipt(*tx_hash)
                .await?
                .ok_or(anyhow!("No transaction receipt"))
        },
        10,
    )
    .await?;

    Ok(receipt)
}

#[instrument(skip_all)]
pub async fn send_raw_tx<P, T>(
    provider: P,
    to: Address,
    data: Bytes,
    chain_id: u8,
    gas: u64,
    gas_fees: GasFees,
    value: U256,
) -> Result<TransactionReceipt>
where
    P: Provider<T, Ethereum> + Clone + WalletProvider<Ethereum>,
    T: Transport + Clone,
{
    let transaction = TransactionRequest {
        to: Some(TxKind::Call(to)),
        gas: Some(gas),
        value: Some(value),
        input: TransactionInput {
            input: Some(data),
            data: None,
        },
        chain_id: Some(chain_id.into()),
        transaction_type: Some(2),
        access_list: None,
        max_fee_per_gas: Some(gas_fees.max_fee_per_gas),
        max_priority_fee_per_gas: Some(gas_fees.max_priority_fee_per_gas),
        ..Default::default()
    };

    let pending = retry(|| provider.send_transaction(transaction.clone()), 3).await?;
    let tx_hash = pending.tx_hash();

    info!("Ethereum transaction {} was sent to the network", tx_hash);

    let receipt = retry(
        || async {
            provider
                .get_transaction_receipt(*tx_hash)
                .await?
                .ok_or(anyhow!("No transaction receipt"))
        },
        10,
    )
    .await?;

    Ok(receipt)
}
