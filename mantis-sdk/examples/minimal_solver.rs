use std::env;

use alloy::primitives::U256;
use alloy::signers::local::PrivateKeySigner;
use anyhow::{Context, Result};
use mantis_sdk::auction::ws::AuctioneerWsClient;
use solana_sdk::signature::{Keypair, Signer};
use tracing::info;

use auctioneer_api::ws::{
    ClientBidMessage, ClientMessage, ClientRegisterMessage, ServerMessage, SignableMessage,
    SolverAddresses,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    // Load .env file if it exists
    dotenv::dotenv().ok();

    // Load configuration
    let solver_id = env::var("SOLVER_ID").unwrap_or_else(|_| "minimal001".to_string());
    let ws_url = env::var("AUCTIONEER_WS_URL").unwrap_or_else(|_| "ws://localhost:8080/auction".to_string());
    
    // Setup signers
    let ethereum_signer = env::var("ETHEREUM_PRIVATE_KEY")
        .context("ETHEREUM_PRIVATE_KEY required")?
        .parse::<PrivateKeySigner>()?;
    
    let solana_keypair = Keypair::from_bytes(
        &bs58::decode(env::var("SOLANA_PRIVATE_KEY").context("SOLANA_PRIVATE_KEY required")?)
            .into_vec()?
    )?;

    // Connect to auctioneer
    let ws_client = AuctioneerWsClient::connect(&ws_url, None).await?;
    info!("Connected to auctioneer at {}", ws_url);

    // Register solver
    let register_msg = ClientRegisterMessage::new(
        solver_id.clone(),
        SolverAddresses {
            ethereum: ethereum_signer.address(),
            solana: solana_keypair.pubkey(),
            base: ethereum_signer.address(),
        },
    )
    .signed(ethereum_signer.clone())?;

    ws_client.send_message(ClientMessage::Register(register_msg)).await?;
    info!("Registered solver: {}", solver_id);

    // Main message loop
    loop {
        match ws_client.receive_message().await {
            Ok(ServerMessage::AuctionStart(auction)) => {
                info!("Auction {} started", auction.intent_id);
                
                // Simple bid: just bid the input amount minus 1%
                let amount_in = U256::from_str_radix(&auction.intent.amount_in, 10)?;
                let bid_amount = amount_in * U256::from(99) / U256::from(100);
                
                let bid = ClientBidMessage::new(
                    solver_id.clone(),
                    auction.intent_id,
                    bid_amount.to_string(),
                )
                .signed(ethereum_signer.clone())?;
                
                ws_client.send_message(ClientMessage::Bid(bid)).await?;
                info!("Placed bid for auction {}", auction.intent_id);
            }
            Ok(ServerMessage::AuctionResult(result)) => {
                if result.won {
                    info!("Won auction {}! Amount: {}", result.intent_id, result.amount);
                    // In a real solver, execute the swap here
                }
            }
            Ok(ServerMessage::Error(e)) => {
                eprintln!("Server error: {} - {}", e.code, e.message);
            }
            Ok(_) => {} // Handle other message types as needed
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
        }
    }

    Ok(())
}