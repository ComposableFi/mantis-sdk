use crate::Chain;
use alloy::primitives::Address;
use anchor_spl::token_2022::spl_token_2022::solana_program::pubkey::Pubkey;
use clap::{Args, FromArgMatches, Parser, Subcommand};
use mantis_common::UserIntent;

#[derive(Parser)]
#[command(name = "mantis-cli", version = "0.1.0", author = "CF Services")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub async fn run(self) -> anyhow::Result<()> {
        match self.command {
            Commands::Intent { action } => match action {
                IntentActions::Submit(cmd) => cmd.run().await,
            },
            Commands::GetQuote(args) => {
                println!("Getting quote:");
                println!("Token In: {}", args.token_in_name);
                println!("Token Out: {}", args.token_out_name);
                println!("Amount In: {}", args.amount_in);
                // Placeholder for get_quote function
                // let res = get_quote(&args.token_in_name, &args.token_out_name, &args.amount_in).await;
                // println!("{}", res.unwrap());
                Ok(())
            }
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Manage intents
    Intent {
        #[command(subcommand)]
        action: IntentActions,
    },
    /// Get a quote for token exchange
    GetQuote(GetQuoteCmd),
}

#[derive(Subcommand)]
enum IntentActions {
    /// Submit an intent to exchange tokens
    Submit(SubmitIntentCmd),
}

#[derive(Args)]
struct IntentSubmitArgs<T>
where
    T: FromArgMatches + Args,
{
    #[command(flatten)]
    swap_args: TokenSwapArgs,
    #[command(flatten)]
    additional: T,
}

#[derive(Args)]
struct TokenSwapArgs {
    #[arg(value_name = "TOKEN_IN_NAME")]
    token_in_name: String,
    #[arg(value_name = "AMOUNT_IN")]
    amount_in: String,
    #[arg(value_name = "TOKEN_OUT_NAME")]
    token_out_name: String,
    #[arg(value_name = "AMOUNT_OUT")]
    amount_out: String,
}

#[derive(Args)]
struct GetQuoteCmd {
    #[arg(value_name = "TOKEN_IN_NAME")]
    token_in_name: String,
    #[arg(value_name = "TOKEN_OUT_NAME")]
    token_out_name: String,
    #[arg(value_name = "AMOUNT_IN")]
    amount_in: String,
}

#[derive(Subcommand)]
enum SubmitIntentNetworkCmd {
    /// Submit intent on Solana network
    Solana(IntentSubmitArgs<crate::solana::cli::SolanaArgs>),
    /// Submit intent on Ethereum network
    Ethereum(IntentSubmitArgs<crate::ethereum::cli::EthereumArgs>),
}

impl SubmitIntentNetworkCmd {
    pub(crate) fn swap_args(&self) -> &TokenSwapArgs {
        match self {
            SubmitIntentNetworkCmd::Solana(args) => &args.swap_args,
            SubmitIntentNetworkCmd::Ethereum(args) => &args.swap_args,
        }
    }
}

#[derive(Args)]
struct SubmitIntentCmd {
    #[command(subcommand)]
    network: SubmitIntentNetworkCmd,
}

impl SubmitIntentCmd {
    pub(crate) async fn run(self) -> anyhow::Result<()> {
        let SubmitIntentCmd { network } = self;

        let exchange_args = network.swap_args();
        match &network {
            SubmitIntentNetworkCmd::Solana(IntentSubmitArgs {
                additional: solana_args,
                ..
            }) => {
                println!("Submitting intent on Solana network:");
                println!(
                    "Token In: {} Amount In: {}",
                    exchange_args.token_in_name, exchange_args.amount_in
                );
                println!(
                    "Token Out: {} Amount Out: {}",
                    exchange_args.token_out_name, exchange_args.amount_out
                );
                if let Some(mnemonic) = solana_args.mnemonic.as_ref() {
                    println!("Using keypair: {}", mnemonic)
                };
                let solana_client = solana_args.build_client().await?;

                let user_intent = UserIntent {
                    token_in: exchange_args.token_in_name.clone(),
                    amount_in: exchange_args.amount_in.clone(),
                    token_out: exchange_args.token_out_name.clone(),
                    amount_out: exchange_args.amount_out.clone(),
                    user_address: solana_client.signer().to_string(),
                };
                let address = Pubkey::default();
                solana_client.submit_intent(user_intent, address).await?;
                Ok(())
            }
            SubmitIntentNetworkCmd::Ethereum(IntentSubmitArgs {
                additional: eth_args,
                ..
            }) => {
                let eth_client = eth_args.build_client().await?;

                let user_intent = UserIntent {
                    token_in: exchange_args.token_in_name.clone(),
                    amount_in: exchange_args.amount_in.clone(),
                    token_out: exchange_args.token_out_name.clone(),
                    amount_out: exchange_args.amount_out.clone(),
                    user_address: eth_client.signer().to_string(),
                };
                let address = Address::default();
                eth_client.submit_intent(user_intent, address).await?;
                Ok(())
            }
        }
    }
}
