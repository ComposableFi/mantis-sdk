use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use alloy::primitives::U256;
use alloy::signers::local::PrivateKeySigner;
use anyhow::{Context, Result};
use mantis_sdk::auction::ws::AuctioneerWsClient;
use solana_sdk::signature::{Keypair, Signer};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use auctioneer_api::ws::{
    ClientBidMessage, ClientMessage, ClientQuoteMessage, ClientRegisterMessage, ClientSolveMessage,
    IntentChain, ServerAuctionResultMessage, ServerAuctionStartMessage, ServerMessage, ServerQuoteMessage,
    SignableMessage, SolverAddresses, SwapIntent,
};

#[derive(Clone)]
struct SolverConfig {
    solver_id: String,
    auctioneer_ws_url: String,
    ethereum_private_key: String,
    solana_private_key: String,
    commission_bps: u16,
}

impl SolverConfig {
    fn from_env() -> Result<Self> {
        Ok(SolverConfig {
            solver_id: env::var("SOLVER_ID").unwrap_or_else(|_| "simple001".to_string()),
            auctioneer_ws_url: env::var("AUCTIONEER_WS_URL")
                .unwrap_or_else(|_| "ws://localhost:8080/auction".to_string()),
            ethereum_private_key: env::var("ETHEREUM_PRIVATE_KEY")
                .context("ETHEREUM_PRIVATE_KEY must be set")?,
            solana_private_key: env::var("SOLANA_PRIVATE_KEY").context("SOLANA_PRIVATE_KEY must be set")?,
            commission_bps: env::var("COMMISSION_BPS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .context("Invalid COMMISSION_BPS")?,
        })
    }
}

#[derive(Clone)]
struct SolverContext {
    config: SolverConfig,
    ethereum_signer: PrivateKeySigner,
    solana_keypair: Arc<Keypair>,
    pending_intents: Arc<Mutex<HashMap<u64, SwapIntent>>>,
}

impl SolverContext {
    async fn new(config: SolverConfig) -> Result<Self> {
        let ethereum_signer = config
            .ethereum_private_key
            .parse::<PrivateKeySigner>()
            .context("Invalid Ethereum private key")?;

        let solana_keypair_bytes = bs58::decode(&config.solana_private_key)
            .into_vec()
            .context("Invalid Solana private key")?;
        let solana_keypair = Arc::new(Keypair::from_bytes(&solana_keypair_bytes)?);

        Ok(SolverContext {
            config,
            ethereum_signer,
            solana_keypair,
            pending_intents: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn get_solver_addresses(&self) -> SolverAddresses {
        SolverAddresses {
            ethereum: self.ethereum_signer.address(),
            solana: self.solana_keypair.as_ref().pubkey(),
            base: self.ethereum_signer.address(),
        }
    }

    async fn calculate_output_amount(&self, intent: &SwapIntent) -> Result<U256> {
        let amount_in = U256::from_str_radix(&intent.amount_in, 10)?;

        let commission = amount_in * U256::from(self.config.commission_bps) / U256::from(10000);
        let output_amount = amount_in - commission;

        info!(
            intent_id = intent.timeout,
            input = %amount_in,
            commission = %commission,
            output = %output_amount,
            "Calculated output amount"
        );

        Ok(output_amount)
    }

    async fn execute_swap(&self, intent: &SwapIntent) -> Result<String> {
        match (intent.src_chain.clone(), intent.dst_chain.clone()) {
            (IntentChain::Ethereum, IntentChain::Ethereum) => {
                info!("Executing Ethereum to Ethereum swap");
                Ok("0x1234567890abcdef".to_string())
            }
            (IntentChain::Solana, IntentChain::Solana) => {
                info!("Executing Solana to Solana swap");
                Ok("5XE5bHJhJ8VZ8Z".to_string())
            }
            (IntentChain::Ethereum, IntentChain::Solana) => {
                info!("Executing Ethereum to Solana cross-chain swap");
                Ok("cross_chain_tx_123".to_string())
            }
            (IntentChain::Solana, IntentChain::Ethereum) => {
                info!("Executing Solana to Ethereum cross-chain swap");
                Ok("cross_chain_tx_456".to_string())
            }
            _ => {
                warn!("Unsupported chain combination");
                Err(anyhow::anyhow!("Unsupported chain combination"))
            }
        }
    }
}

async fn handle_auction_start(
    ctx: Arc<SolverContext>,
    ws_client: Arc<AuctioneerWsClient>,
    msg: ServerAuctionStartMessage,
) -> Result<()> {
    info!(
        intent_id = msg.intent_id,
        src_chain = ?msg.intent.src_chain,
        dst_chain = ?msg.intent.dst_chain,
        "Received auction start"
    );

    let output_amount = ctx.calculate_output_amount(&msg.intent).await?;

    let min_amount_out = U256::from_str_radix(&msg.intent.amount_out, 10)?;
    if output_amount < min_amount_out {
        info!(
            intent_id = msg.intent_id,
            calculated = %output_amount,
            required = %min_amount_out,
            "Skipping unprofitable auction"
        );
        return Ok(());
    }

    ctx.pending_intents
        .lock()
        .await
        .insert(msg.intent_id, msg.intent.clone());

    let bid_msg = ClientBidMessage::new(
        ctx.config.solver_id.clone(),
        msg.intent_id,
        output_amount.to_string(),
    )
    .signed(ctx.ethereum_signer.clone())?;

    ws_client
        .send_message(ClientMessage::Bid(bid_msg))
        .await
        .context("Failed to send bid")?;

    info!(
        intent_id = msg.intent_id,
        amount = %output_amount,
        "Sent bid"
    );

    Ok(())
}

async fn handle_auction_result(
    ctx: Arc<SolverContext>,
    ws_client: Arc<AuctioneerWsClient>,
    msg: ServerAuctionResultMessage,
) -> Result<()> {
    if !msg.won {
        info!(intent_id = msg.intent_id, "Lost auction");
        ctx.pending_intents.lock().await.remove(&msg.intent_id);
        return Ok(());
    }

    info!(intent_id = msg.intent_id, amount = msg.amount, "Won auction!");

    let intent = ctx
        .pending_intents
        .lock()
        .await
        .remove(&msg.intent_id)
        .context("Intent not found in pending intents")?;

    match ctx.execute_swap(&intent).await {
        Ok(tx_hash) => {
            let solve_msg =
                ClientSolveMessage::new(ctx.config.solver_id.clone(), msg.intent_id, tx_hash.clone())
                    .signed(ctx.ethereum_signer.clone())?;

            ws_client
                .send_message(ClientMessage::Solve(solve_msg))
                .await
                .context("Failed to send solve message")?;

            info!(intent_id = msg.intent_id, tx_hash = tx_hash, "Sent solve message");
        }
        Err(e) => {
            error!(
                intent_id = msg.intent_id,
                error = %e,
                "Failed to execute swap"
            );
        }
    }

    Ok(())
}

async fn handle_quote_request(
    ctx: Arc<SolverContext>,
    ws_client: Arc<AuctioneerWsClient>,
    msg: ServerQuoteMessage,
) -> Result<()> {
    info!(
        request_id = %msg.request_id,
        "Received quote request"
    );

    let output_amount = ctx.calculate_output_amount(&msg.intent).await?;

    let quote_response = ClientQuoteMessage {
        request_id: msg.request_id,
        src_chain: msg.intent.src_chain.to_string(),
        dst_chain: msg.intent.dst_chain.to_string(),
        solver_id: ctx.config.solver_id.clone(),
        token_in: msg.intent.token_in.clone(),
        amount_in: msg.intent.amount_in.clone(),
        token_out: msg.intent.token_out.clone(),
        amount_out: output_amount.to_string(),
    };

    ws_client
        .send_message(ClientMessage::Quote(quote_response))
        .await
        .context("Failed to send quote response")?;

    Ok(())
}

async fn handle_server_message(
    ctx: Arc<SolverContext>,
    ws_client: Arc<AuctioneerWsClient>,
    message: ServerMessage,
) -> Result<()> {
    match message {
        ServerMessage::AuctionStart(msg) => {
            tokio::spawn(handle_auction_start(ctx, ws_client, msg));
        }
        ServerMessage::AuctionResult(msg) => {
            tokio::spawn(handle_auction_result(ctx, ws_client, msg));
        }
        ServerMessage::Quote(msg) => {
            tokio::spawn(handle_quote_request(ctx, ws_client, msg));
        }
        ServerMessage::UnlockedFunds(msg) => {
            info!(
                intent_id = msg.intent_id,
                amount = msg.amount_in,
                "Funds unlocked notification"
            );
        }
        ServerMessage::Error(msg) => {
            error!(
                code = msg.code,
                message = msg.message,
                request_id = ?msg.request_id,
                "Server error"
            );
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    // Load .env file if it exists
    dotenv::dotenv().ok();

    info!("Starting simple solver example");

    let config = SolverConfig::from_env()?;
    let ctx = Arc::new(SolverContext::new(config.clone()).await?);

    let ws_client = Arc::new(
        AuctioneerWsClient::connect(&config.auctioneer_ws_url, None)
            .await
            .context("Failed to create WebSocket client")?,
    );

    let register_msg = ClientRegisterMessage::new(config.solver_id.clone(), ctx.get_solver_addresses())
        .signed(ctx.ethereum_signer.clone())?;

    ws_client
        .send_message(ClientMessage::Register(register_msg))
        .await
        .context("Failed to send registration")?;

    info!(solver_id = config.solver_id, "Sent registration");

    loop {
        tokio::select! {
            Ok(message) = ws_client.receive_message() => {
                if let Err(e) = handle_server_message(ctx.clone(), ws_client.clone(), message).await {
                    error!(error = %e, "Failed to handle server message");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("Shutting down solver");
                break;
            }
        }
    }

    info!("Solver shut down");

    Ok(())
}

