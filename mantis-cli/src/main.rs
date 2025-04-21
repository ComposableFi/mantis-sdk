use std::str::FromStr;
use std::sync::Arc;

use alloy::network::EthereumWallet;
use alloy::primitives::{Address, U256};
use alloy::providers::ProviderBuilder;
use alloy::signers::local::PrivateKeySigner;
use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand};
use mantis_sdk::{ethereum, solana};
use num::BigUint;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

/// A CLI utility to interact with Mantis smart contracts
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Solana private key
    #[arg(long, env = "SOLANA_SIGNER", required = true)]
    solana_signer: String,

    /// Ethereum private key
    #[arg(long, env = "ETHEREUM_SIGNER", required = true)]
    ethereum_signer: String,

    /// Solana network connection URL
    #[arg(
        long,
        env = "SOLANA_URL",
        default_value = "https://api.mainnet-beta.solana.com"
    )]
    solana_url: String,

    /// Ethereum network connection URL
    #[arg(long, env = "ETHEREUM_URL", default_value = "https://rpc.flashbots.net")]
    ethereum_url: String,

    /// Solana Escrow program address
    #[arg(
        long,
        env = "SOLANA_PROGRAM",
        default_value = "7D41jAGYFTeJhd5bFzJgY4TwrWL7zr7oC454e5yqjw4Q"
    )]
    solana_program: String,

    /// Ethereum Escrow program address
    #[arg(
        long,
        env = "ETHEREUM_PROGRAM",
        default_value = "0xaf55771e9cd32f93532670ef358c8703d598505c"
    )]
    ethereum_program: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Swap(SwapArgs),
    Cancel(CancelArgs),
}

#[derive(Args, Debug)]
struct SwapArgs {
    /// Source chain name (e.g. solana, ethereum)
    #[arg(long, required = true)]
    src_chain: String,

    /// Destination chain name (e.g. solana, ethereum)
    #[arg(long)]
    dst_chain: Option<String>,

    /// Address of the destination user
    #[arg(long)]
    dst_user: Option<String>,

    /// Input token address
    #[arg(long, required = true)]
    token_in: String,

    /// Output token address
    #[arg(long, required = true)]
    token_out: String,

    /// Amount of input tokens in base units
    #[arg(long, required = true)]
    amount_in: u64,

    /// Expected minimum amount of output tokens in base units
    #[arg(long, default_value_t = 1)]
    amount_out: u64,

    /// Swap timeout in seconds
    #[arg(long, default_value_t = 180)]
    timeout: u64,
}

#[derive(Args, Debug)]
struct CancelArgs {
    /// Source chain name (e.g. solana, ethereum)
    #[arg(long)]
    src_chain: String,

    /// The ID of the intent to be canceled
    #[arg(long)]
    intent_id: u64,

    /// Input token address
    #[arg(long)]
    token_in: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    dotenv::dotenv()?;
    let cli = Cli::parse();

    let eth_signer = Arc::new(PrivateKeySigner::from_str(&cli.ethereum_signer)?);
    let eth_client = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(EthereumWallet::from(eth_signer.clone()))
        .on_http(cli.ethereum_url.parse()?);

    let sol_signer = Arc::new(Keypair::from_base58_string(&cli.solana_signer));
    let sol_client = RpcClient::new_with_commitment(cli.solana_url.clone(), CommitmentConfig::confirmed());

    match cli.command {
        Commands::Swap(args) => match args.src_chain.as_str() {
            "solana" => {
                let program_id = Pubkey::from_str(&cli.solana_program).context("solana_program")?;
                let token_in = Pubkey::from_str(&args.token_in).context("token_in")?;
                let amount_in = BigUint::from(args.amount_in);
                let token_out = Pubkey::from_str(&args.token_out)
                    .context("token_out")?
                    .to_string();
                let amount_out = BigUint::from(args.amount_out);
                let dst_user = if let Some(dst_user) = args.dst_user {
                    Pubkey::from_str(&dst_user)
                        .map(|dst_user| dst_user.to_string())
                        .or(Address::from_str(&dst_user).map(|dst_user| dst_user.to_checksum(None)))
                        .context("dst_user")?
                } else {
                    sol_signer.pubkey().to_string()
                };

                let dst_chain_id = match args.dst_chain {
                    Some(dst_chain) if dst_chain == "ethereum" => 1,
                    _ => 2,
                };

                let signature = solana::escrow_funds(
                    &sol_client,
                    sol_signer,
                    program_id,
                    token_in,
                    amount_in,
                    token_out,
                    amount_out,
                    dst_user,
                    dst_chain_id,
                    args.timeout,
                    false,
                )
                .await
                .context("Escrow funds operation failed")?;

                println!("Transaction: {}", signature);
            }
            "ethereum" => {
                let escrow_address = Address::from_str(&cli.ethereum_program)?;
                let token_in = Address::from_str(&args.token_in).context("token_in")?;
                let amount_in = U256::from(args.amount_in);
                let token_out = Address::from_str(&args.token_out)
                    .context("token_out")?
                    .to_checksum(None);
                let amount_out = U256::from(args.amount_out);
                let dst_user = if let Some(dst_user) = args.dst_user {
                    Address::from_str(&dst_user)
                        .map(|dst_user| dst_user.to_checksum(None))
                        .or(Pubkey::from_str(&dst_user).map(|dst_user| dst_user.to_string()))
                        .context("dst_user")?
                } else {
                    eth_signer.address().to_checksum(None)
                };

                let dst_chain_id = match args.dst_chain {
                    Some(dst_chain) if dst_chain == "solana" => 2,
                    _ => 1,
                };

                let receipt = ethereum::escrow_funds(
                    eth_client,
                    escrow_address,
                    token_in,
                    amount_in,
                    token_out,
                    amount_out,
                    dst_user,
                    dst_chain_id,
                    args.timeout,
                    false,
                )
                .await
                .context("Escrow funds operation failed")?;

                println!("Transaction: {}", receipt.transaction_hash);
            }
            _ => {
                return Err(anyhow!("Invalid src_chain argument"));
            }
        },
        Commands::Cancel(args) => match args.src_chain.as_str() {
            "solana" => {
                let program_id = Pubkey::from_str(&cli.solana_program).context("solana_program")?;

                let token_in = Pubkey::from_str(
                    &args
                        .token_in
                        .ok_or(anyhow!("token_in required for Solana intent"))?,
                )
                .context("token_in")?;

                let signature = solana::cancel_intent(
                    &sol_client,
                    sol_signer.clone(),
                    program_id,
                    args.intent_id,
                    token_in,
                    sol_signer.pubkey(),
                )
                .await
                .context("Cancel intent operation failed")?;

                println!("Transaction: {}", signature);
            }
            "ethereum" => {
                let escrow_address = Address::from_str(&cli.ethereum_program)?;

                let receipt = ethereum::cancel_intent(eth_client, escrow_address, args.intent_id)
                    .await
                    .context("Cancel intent operation failed")?;

                println!("Transaction: {}", receipt.transaction_hash);
            }
            _ => {
                return Err(anyhow!("Invalid src_chain argument"));
            }
        },
    }

    Ok(())
}
