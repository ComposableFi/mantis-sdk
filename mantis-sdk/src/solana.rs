use std::sync::Arc;

use anchor_client::{Client, Cluster};
use anchor_lang::prelude::declare_program;
use anchor_spl::associated_token;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use num::BigUint;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::address_lookup_table::state::AddressLookupTable;
use solana_sdk::address_lookup_table::AddressLookupTableAccount;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::{v0, VersionedMessage};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signer::Signer;
use solana_sdk::system_program;
use solana_sdk::sysvar::{clock, rent};
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use spl_token::instruction::transfer;
use spl_token::native_mint;
use thiserror::Error;
use tracing::{info, instrument};

use crate::{random_intent_id, retry};

#[derive(Error, Debug)]
pub enum SolanaError {
    #[error("Solana RPC client error: {0}")]
    RpcClientError(String), // For errors from RpcClient interactions

    #[error("Anchor client error: {0}")]
    AnchorClientError(String), // For errors from anchor_client::Client

    #[error("Program error: {0}")]
    ProgramInteractionError(String), // For general on-chain program interaction issues

    #[error("Transaction failed: {signature} with reason: {reason}")]
    TransactionFailed { signature: String, reason: String },

    #[error("Transaction confirmation timed out for signature: {signature}")]
    TransactionConfirmationTimeout { signature: String },

    #[error("Failed to build transaction: {0}")]
    TransactionBuildError(String),

    #[error("Account not found: {account_pubkey}")]
    AccountNotFound { account_pubkey: String },

    #[error("Data conversion or parsing error: {0}")]
    ConversionError(String), // For BigUint, Pubkey parsing, etc.

    #[error("Missing expected account data for: {account_description}")]
    MissingAccountData { account_description: String },

    #[error("Token operation failed for mint {mint_pubkey}: {details}")]
    TokenOperationError { mint_pubkey: String, details: String },

    #[error("An underlying anyhow error occurred: {0}")]
    Anyhow(#[from] anyhow::Error), // Fallback for quick migration
}

declare_program!(escrow);

#[allow(unused)]
use escrow::{accounts::Intent, client::accounts, client::args, types, utils};

pub const ESCROW_SEED: &[u8] = b"escrow";
pub const FEE_SEED: &[u8] = b"fee";
pub const SOL_SEED: &[u8] = b"lamport";
pub const INTENT_SEED: &[u8] = b"intent";
pub const INTENT_CHAIN_ID: u8 = 2;

#[derive(Debug, Clone, Default)]
pub struct VTransaction {
    pub instructions: Vec<Instruction>,
    pub address_lookup_table: Vec<AddressLookupTableAccount>,
}

impl VTransaction {
    pub fn new(instructions: Vec<Instruction>, address_lookup_table: Vec<AddressLookupTableAccount>) -> Self {
        VTransaction {
            instructions,
            address_lookup_table,
        }
    }
}

#[instrument(skip_all)]
pub async fn escrow_funds(
    client: &RpcClient,
    payer: Arc<Keypair>,
    program_id: Pubkey,
    token_in: Pubkey,
    amount_in: BigUint,
    token_out: String,
    amount_out: BigUint,
    dst_user: String,
    dst_chain_id: u8,
    timeout_sec: u64,
    ai_agent: bool,
) -> Result<Signature, SolanaError> {
    let cluster = Cluster::Custom(client.url(), String::default());
    let client_anchor = Client::new_with_options(cluster, payer.clone(), CommitmentConfig::processed());
    let program = client_anchor
        .program(program_id)
        .map_err(|e| SolanaError::AnchorClientError(e.to_string()))?;

    let src_user = payer.pubkey();
    let timestamp = Utc::now().timestamp() as u64;
    let timeout = timestamp + timeout_sec;
    let intent_id = random_intent_id();

    let token_in_mint = if token_in == Pubkey::default() {
        spl_token::native_mint::ID
    } else {
        token_in
    };

    let (escrow_pda, _) = Pubkey::find_program_address(&[ESCROW_SEED], &program_id);
    let (escrow_sol_pda, _) = Pubkey::find_program_address(&[ESCROW_SEED, SOL_SEED], &program_id);
    let (fee_sol_pda, _) = Pubkey::find_program_address(&[FEE_SEED, SOL_SEED], &program_id);
    let (fee_pda, _) = Pubkey::find_program_address(&[FEE_SEED], &program_id);
    let (intent_pda, _) = Pubkey::find_program_address(&[INTENT_SEED, &intent_id.to_le_bytes()], &program_id);
    let (fee_token_in_pda, _) =
        Pubkey::find_program_address(&[FEE_SEED, token_in_mint.as_ref()], &program_id);

    let (user_token_in_ata_ix, user_token_in_ata) =
        create_associated_token_account_ix(client, payer.clone(), src_user, token_in_mint)
            .await
            .map_err(SolanaError::Anyhow)?;

    let (escrow_token_in_ata_ix, escrow_token_in_ata) =
        create_associated_token_account_ix(client, payer.clone(), escrow_pda, token_in_mint)
            .await
            .map_err(SolanaError::Anyhow)?;

    let amount_in_u64 = amount_in
        .try_into()
        .map_err(|e| SolanaError::ConversionError(format!("Failed to convert amount_in to u64: {}", e)))?;

    let new_intent = types::NewIntent {
        intent_id,
        src_user,
        dst_user,
        token_in,
        amount_in: amount_in_u64,
        token_out,
        amount_out: amount_out.to_str_radix(10),
        timeout,
        ai_agent,
        dst_chain_id,
    };

    let escrow_accounts = accounts::EscrowFunds {
        signer: src_user,
        user_token_in_ata,
        escrow_pda,
        escrow_sol_pda,
        token_in_mint,
        escrow_token_in_ata,
        intent_pda,
        fee_sol_pda,
        clock: clock::ID,
        token_program: anchor_spl::token::ID,
        token_2022_program: anchor_spl::token_2022::ID,
        associated_token_program: associated_token::ID,
        system_program: system_program::ID,
        fee_token_in_pda,
        fee_pda,
    };

    let escrow_args = args::EscrowFunds {
        new_intent: new_intent.clone(),
    };

    let escrow_instructions = program
        .request()
        .instruction(ComputeBudgetInstruction::set_compute_unit_limit(100_000))
        .instruction(ComputeBudgetInstruction::set_compute_unit_price(100_000))
        .instruction(user_token_in_ata_ix)
        .instruction(escrow_token_in_ata_ix)
        .accounts(escrow_accounts)
        .args(escrow_args)
        .instructions()
        .map_err(|e| {
            SolanaError::TransactionBuildError(format!("Failed to build escrow instructions: {}", e))
        })?;

    let recent_blockhash = retry(|| client.get_latest_blockhash(), 3)
        .await
        .map_err(|e| SolanaError::RpcClientError(format!("Failed to get latest blockhash: {}", e)))?;

    let escrow_transaction = Transaction::new_signed_with_payer(
        &escrow_instructions,
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let signature = retry(|| client.send_and_confirm_transaction(&escrow_transaction), 3)
        .await
        .map_err(|e| SolanaError::TransactionFailed {
            signature: "unknown".to_string(),
            reason: format!("Failed to send and confirm transaction: {}", e),
        })?;

    Ok(signature)
}

#[instrument(skip_all)]
pub async fn cancel_intent(
    client: &RpcClient,
    payer: Arc<Keypair>,
    program_id: Pubkey,
    intent_id: u64,
    mut token_in: Pubkey,
    src_user: Pubkey,
) -> Result<Signature, SolanaError> {
    let cluster = Cluster::Custom(client.url(), String::default());
    let client_anchor = Client::new_with_options(cluster, payer.clone(), CommitmentConfig::confirmed());
    let program = client_anchor
        .program(program_id)
        .map_err(|e| SolanaError::AnchorClientError(e.to_string()))?;

    if token_in == Pubkey::default() {
        token_in = native_mint::ID;
    }

    let (escrow_pda, _) = Pubkey::find_program_address(&[ESCROW_SEED], &program.id());
    let (escrow_sol_pda, _) = Pubkey::find_program_address(&[ESCROW_SEED, SOL_SEED], &program.id());
    let (fee_sol_pda, _) = Pubkey::find_program_address(&[FEE_SEED, SOL_SEED], &program.id());
    let (fee_token_in_pda, _) = Pubkey::find_program_address(&[FEE_SEED, token_in.as_ref()], &program.id());
    let (intent_pda, _) =
        Pubkey::find_program_address(&[INTENT_SEED, &intent_id.to_le_bytes()], &program.id());

    let (user_token_in_ata_ix, user_token_in_ata) =
        create_associated_token_account_ix(client, payer.clone(), src_user, token_in)
            .await
            .map_err(SolanaError::Anyhow)?;

    let (escrow_token_in_ata_ix, escrow_token_in_ata) =
        create_associated_token_account_ix(client, payer.clone(), escrow_pda, token_in)
            .await
            .map_err(SolanaError::Anyhow)?;

    let cancel_accounts = accounts::CancelIntent {
        signer: payer.pubkey(),
        escrow_pda,
        escrow_sol_pda,
        intent_pda,
        fee_sol_pda,
        token_in_mint: token_in,
        src_user,
        user_token_in_ata,
        escrow_token_in_ata,
        fee_token_in_pda,
        clock: clock::ID,
        token_2022_program: anchor_spl::token_2022::ID,
        token_program: anchor_spl::token::ID,
        associated_token_program: associated_token::ID,
        system_program: system_program::ID,
    };

    let cancel_args = args::CancelIntent { intent_id };

    let cancel_instructions = program
        .request()
        .instruction(user_token_in_ata_ix)
        .instruction(escrow_token_in_ata_ix)
        .accounts(cancel_accounts)
        .args(cancel_args)
        .instructions()
        .map_err(|e| {
            SolanaError::TransactionBuildError(format!("Failed to build cancel intent instructions: {}", e))
        })?;

    let recent_blockhash = retry(|| client.get_latest_blockhash(), 3)
        .await
        .map_err(|e| SolanaError::RpcClientError(format!("Failed to get latest blockhash: {}", e)))?;

    let cancel_transaction = Transaction::new_signed_with_payer(
        &cancel_instructions,
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let signature = retry(|| client.send_and_confirm_transaction(&cancel_transaction), 3)
        .await
        .map_err(|e| SolanaError::TransactionFailed {
            signature: "unknown".to_string(),
            reason: format!("Failed to cancel intent {}: {}", intent_id, e),
        })?;

    Ok(signature)
}

#[instrument(skip_all)]
pub async fn initialize(client: &RpcClient, payer: Arc<Keypair>, program_id: Pubkey) -> Result<Signature> {
    let cluster = Cluster::Custom(client.url(), String::default());
    let client_anchor = Client::new_with_options(cluster, payer.clone(), CommitmentConfig::processed());
    let program = client_anchor.program(program_id)?;

    let (escrow_pda, _) = Pubkey::find_program_address(&[ESCROW_SEED], &program_id);
    let (escrow_sol_pda, _) = Pubkey::find_program_address(&[ESCROW_SEED, SOL_SEED], &program_id);
    let (fee_pda, _) = Pubkey::find_program_address(&[FEE_SEED], &program_id);
    let (fee_sol_pda, _) = Pubkey::find_program_address(&[FEE_SEED, SOL_SEED], &program_id);

    let initialize_accounts = accounts::Initialize {
        authority: payer.pubkey(),
        escrow_pda,
        escrow_sol_pda,
        fee_pda,
        fee_sol_pda,
        rent: rent::ID,
        system_program: system_program::ID,
    };

    let initialize_args = args::Initialize {};

    let initialize_ix = program
        .request()
        .accounts(initialize_accounts)
        .args(initialize_args)
        .instructions()?;

    let recent_blockhash = retry(|| client.get_latest_blockhash(), 3).await?;

    let initialize_tx = Transaction::new_signed_with_payer(
        &initialize_ix,
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let signature = retry(|| client.send_and_confirm_transaction(&initialize_tx), 3).await?;
    Ok(signature)
}

#[instrument(skip_all)]
pub async fn create_associated_token_account(
    client: &RpcClient,
    payer: Arc<Keypair>,
    user: Pubkey,
    token_mint: Pubkey,
) -> Result<(Signature, Pubkey)> {
    let (create_associated_account_ix, ata) =
        create_associated_token_account_ix(client, payer.clone(), user, token_mint).await?;

    let recent_blockhash = retry(|| client.get_latest_blockhash(), 3).await?;

    let create_associated_account_tx = Transaction::new_signed_with_payer(
        &[create_associated_account_ix],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );

    let signature = retry(
        || client.send_and_confirm_transaction(&create_associated_account_tx),
        3,
    )
    .await?;

    Ok((signature, ata))
}

pub async fn create_associated_token_account_ix(
    client: &RpcClient,
    payer: Arc<Keypair>,
    user: Pubkey,
    token_mint: Pubkey,
) -> Result<(Instruction, Pubkey)> {
    let token_program_id = get_token_program_id(client, &token_mint).await?;

    let ata = get_associated_token_address_with_program_id(&user, &token_mint, &token_program_id);

    let instructions =
        create_associated_token_account_idempotent(&payer.pubkey(), &user, &token_mint, &token_program_id);

    Ok((instructions, ata))
}

#[instrument(skip_all)]
pub async fn get_transaction_info(
    client: &RpcClient,
    signature: Signature,
) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
    let config = RpcTransactionConfig {
        encoding: Some(solana_transaction_status::UiTransactionEncoding::Json),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };
    let transaction_info = retry(|| client.get_transaction_with_config(&signature, config), 3).await?;

    Ok(transaction_info)
}

#[instrument(skip_all)]
pub async fn get_lookup_table_accounts(
    client: &RpcClient,
    lookup_table_addresses: &[Pubkey],
) -> Result<Vec<AddressLookupTableAccount>> {
    let lookup_table_accounts = retry(|| client.get_multiple_accounts(lookup_table_addresses), 3).await?;

    lookup_table_addresses
        .iter()
        .zip(lookup_table_accounts.into_iter())
        .map(|(pubkey, account)| {
            let acc = account.ok_or(anyhow!("Missing ALT account data"))?;
            let lookup_table = AddressLookupTable::deserialize(&acc.data)
                .map_err(|e| anyhow!("Failed to deserialize ALT account: {:#}", e))?;
            Ok(AddressLookupTableAccount {
                key: *pubkey,
                addresses: lookup_table.addresses.to_vec(),
            })
        })
        .collect()
}

#[instrument(skip_all)]
pub async fn transfer_spl_token(
    client: &RpcClient,
    sender: Arc<Keypair>,
    recipient: Pubkey,
    token_mint: Pubkey,
    amount: u64,
) -> Result<Signature> {
    let transfer_spl_token_ix =
        transfer_spl_token_ix(client, sender.clone(), recipient, token_mint, amount).await?;

    let recent_blockhash = retry(|| client.get_latest_blockhash(), 3).await?;

    let transfer_spl_token_tx = Transaction::new_signed_with_payer(
        &transfer_spl_token_ix,
        Some(&sender.pubkey()),
        &[&*sender],
        recent_blockhash,
    );

    let signature = retry(|| client.send_and_confirm_transaction(&transfer_spl_token_tx), 3).await?;
    Ok(signature)
}

pub async fn transfer_spl_token_ix(
    client: &RpcClient,
    sender: Arc<Keypair>,
    recipient: Pubkey,
    token_mint: Pubkey,
    amount: u64,
) -> Result<Vec<Instruction>> {
    let token_program_id = get_token_program_id(client, &token_mint).await?;
    let sender_ata =
        get_associated_token_address_with_program_id(&sender.pubkey(), &token_mint, &token_program_id);
    let recipient_ata =
        get_associated_token_address_with_program_id(&recipient, &token_mint, &token_program_id);

    let mut transfer_instructions: Vec<Instruction> = vec![];
    if client.get_account(&recipient_ata).await.is_err() {
        transfer_instructions.push(create_associated_token_account_idempotent(
            &sender.pubkey(),
            &recipient,
            &token_mint,
            &token_program_id,
        ));
    }

    let transfer_instruction = transfer(
        &token_program_id,
        &sender_ata,
        &recipient_ata,
        &sender.pubkey(),
        &[],
        amount,
    )?;
    transfer_instructions.push(transfer_instruction.clone());

    Ok(transfer_instructions)
}

#[instrument(skip_all)]
pub async fn submit_through_rpc(
    client: &RpcClient,
    payer: Arc<Keypair>,
    transaction: &VTransaction,
    legacy_transaction: bool,
) -> Result<Signature> {
    if legacy_transaction {
        let transaction = Transaction::new_signed_with_payer(
            &transaction.instructions,
            Some(&payer.pubkey()),
            &[&*payer],
            retry(|| client.get_latest_blockhash(), 3).await?,
        );
        retry(|| client.send_and_confirm_transaction(&transaction), 3)
            .await
            .context("Failed to send legacy transaction")
    } else {
        let message = v0::Message::try_compile(
            &payer.pubkey(),
            &transaction.instructions,
            &transaction.address_lookup_table,
            retry(|| client.get_latest_blockhash(), 3).await?,
        )?;
        let transaction = VersionedTransaction::try_new(VersionedMessage::V0(message), &[&payer])?;
        retry(|| client.send_and_confirm_transaction(&transaction), 3)
            .await
            .context("Failed to send versioned transaction")
    }
}

#[instrument(skip_all)]
pub async fn submit_through_rpc_multiple(
    client: &RpcClient,
    payer: Arc<Keypair>,
    transactions: Vec<VTransaction>,
) -> Result<Vec<Signature>> {
    let mut signatures: Vec<Signature> = vec![];
    for (i, transaction) in transactions.iter().enumerate() {
        info!("Sending transaction {} of {} via RPC", i + 1, transactions.len());
        let signature = submit_through_rpc(client, payer.clone(), transaction, false)
            .await
            .map_err(|err| {
                anyhow!(
                    "Failed to submit transaction {} of {} via RPC: {:#}",
                    i + 1,
                    transactions.len(),
                    err,
                )
            })?;
        signatures.push(signature);
    }
    Ok(signatures)
}

#[instrument(skip_all)]
pub async fn get_token_program_id(client: &RpcClient, token_mint: &Pubkey) -> Result<Pubkey, SolanaError> {
    match retry(|| client.get_account(token_mint), 3)
        .await
        .map_err(|_| SolanaError::AccountNotFound {
            account_pubkey: token_mint.to_string(),
        })? {
        account if account.owner == spl_token_2022::ID => Ok(spl_token_2022::ID),
        account if account.owner == spl_token::ID => Ok(spl_token::ID),
        _ => Err(SolanaError::TokenOperationError {
            mint_pubkey: token_mint.to_string(),
            details: format!("Could not determine token program ID for mint {}", token_mint),
        }),
    }
}
