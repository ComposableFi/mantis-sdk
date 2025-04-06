#![allow(unused)]
use anchor_client::{Client as AnchorClient, Cluster, Program};
use anchor_lang::prelude::*;
use anchor_spl::associated_token;
use anchor_spl::token::Mint;
use anyhow::{anyhow, Result};
use base64::{self, Engine};
use num::{BigInt, BigUint};
use rand::Rng;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signer::{SeedDerivable, Signer};
use solana_sdk::system_instruction;
use solana_sdk::sysvar::{clock, rent};
use solana_sdk::transaction::Transaction;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, system_program};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

declare_program!(escrow);

use escrow::{accounts::Intent, client::accounts, client::args, types, utils};

pub const ESCROW_SEED: &[u8] = b"escrow";
pub const FEE_SEED: &[u8] = b"fee";
pub const SOL_SEED: &[u8] = b"lamport";
pub const INTENT_SEED: &[u8] = b"intent";
pub const INTENT_CHAIN_ID: u8 = 2;
pub const SOLANA_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
pub const SOLANA_RPC_URL_WS: &str = "wss://api.mainnet-beta.solana.com";

pub async fn escrow_funds(
    signer: Arc<Keypair>,
    token_in: Pubkey,
    amount_in: BigUint,
    token_out: String,
    amount_out: BigUint,
    dst_user: String,
    dst_chain_id: u8,
    spinner: bool,
) -> Result<Signature> {
    let client = Arc::new(RpcClient::new_with_commitment(
        SOLANA_RPC_URL.into(),
        CommitmentConfig::processed(),
    ));
    let cluster = Cluster::Custom(SOLANA_RPC_URL.into(), SOLANA_RPC_URL_WS.into());
    let client_anchor =
        AnchorClient::new_with_options(cluster, signer.clone(), CommitmentConfig::processed());
    let escrow = Arc::new(client_anchor.program(escrow::ID)?);

    let payer = signer.clone();
    let src_user = payer.pubkey();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let timeout = timestamp + 360;
    let intent_id = random_intent_id();

    let token_in_mint = if token_in == Pubkey::default() {
        spl_token::native_mint::ID
    } else {
        token_in
    };
    let (escrow_pda, _) = Pubkey::find_program_address(&[ESCROW_SEED], &escrow.id());
    let (escrow_sol_pda, _) = Pubkey::find_program_address(&[ESCROW_SEED, SOL_SEED], &escrow.id());
    let (fee_sol_pda, _) = Pubkey::find_program_address(&[FEE_SEED, SOL_SEED], &escrow.id());
    let (fee_pda, _) = Pubkey::find_program_address(&[FEE_SEED], &escrow.id());
    let (intent_pda, _) =
        Pubkey::find_program_address(&[INTENT_SEED, &intent_id.to_le_bytes()], &escrow.id());
    let (fee_token_in_pda, _) =
        Pubkey::find_program_address(&[FEE_SEED, token_in_mint.as_ref()], &escrow.id());

    let user_token_in_ata = get_associated_token_address(&src_user, &token_in_mint);
    let signature = create_associated_token_account(
        client.clone(),
        escrow.clone(),
        signer.clone(),
        src_user,
        token_in_mint,
    )
    .await?;

    let escrow_token_in_ata = get_associated_token_address(&escrow_pda, &token_in_mint);
    let signature = create_associated_token_account(
        client.clone(),
        escrow.clone(),
        signer.clone(),
        escrow_pda,
        token_in_mint,
    )
    .await?;

    let new_intent = types::NewIntent {
        intent_id,
        src_user,
        dst_user,
        token_in,
        amount_in: amount_in.try_into()?,
        token_out,
        amount_out: amount_out.to_str_radix(10),
        timeout,
        ai_agent: false,
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

    let escrow_instructions = escrow
        .request()
        .instruction(ComputeBudgetInstruction::set_compute_unit_limit(100_000))
        .instruction(ComputeBudgetInstruction::set_compute_unit_price(100_000))
        .accounts(escrow_accounts)
        .args(escrow_args)
        .instructions()?;

    let recent_blockhash = client.get_latest_blockhash().await?;

    let escrow_transaction = Transaction::new_signed_with_payer(
        &escrow_instructions,
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let signature = if spinner {
        client
            .send_and_confirm_transaction_with_spinner(&escrow_transaction)
            .await?
    } else {
        client
            .send_and_confirm_transaction(&escrow_transaction)
            .await?
    };
    Ok(signature)
}

pub async fn create_associated_token_account(
    client: Arc<RpcClient>,
    escrow: Arc<Program<Arc<Keypair>>>,
    signer: Arc<Keypair>,
    user: Pubkey,
    token_mint: Pubkey,
) -> Result<Signature> {
    let payer = signer.clone();
    let token_program_id = get_token_program_id(client.clone(), &token_mint).await?;
    let create_associated_account_ix = create_associated_token_account_idempotent(
        &payer.pubkey(),
        &user,
        &token_mint,
        &token_program_id,
    );
    let recent_blockhash = client.get_latest_blockhash().await?;
    let create_associated_account_tx = Transaction::new_signed_with_payer(
        &[create_associated_account_ix],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    );
    let signature = client
        .send_and_confirm_transaction(&create_associated_account_tx)
        .await?;
    Ok(signature)
}

async fn get_token_program_id(rpc_client: Arc<RpcClient>, token_mint: &Pubkey) -> Result<Pubkey> {
    let mint_account = rpc_client.get_account(token_mint).await?;

    match mint_account.owner {
        spl_token_2022::ID => Ok(spl_token_2022::ID),
        spl_token::ID => Ok(spl_token::ID),
        _ => Err(anyhow!(
            "Token mint is not owned by Token or Token2022 program"
        )),
    }
}

pub fn random_intent_id() -> u64 {
    rand::thread_rng().gen_range(100_000_000_000..=999_999_999_999)
}
