use alloy::primitives::Address;
use auctioneer_api::ws::{
    ClientQuoteMessage, ClientRegisterMessage, IntentChain as ApiIntentChain, ServerAuctionStartMessage,
    ServerErrorMessage, ServerMessage, SolverAddresses, SwapIntent,
};
use mantis_sdk::auction::{ws::ClientConfig as AuctionClientConfig, ws::ConnectionState, AuctioneerWsClient};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::timeout;
use tracing::info;
use tracing_test::traced_test;
use uuid::Uuid;

mod mock_auctioneer;

use mock_auctioneer::{
    ClientMessageMatcher, ClientMessageTypeMatcher, MockAuctioneer, MockServerConfig, ScriptedAction,
};

#[tokio::test]
#[traced_test]
async fn test_simple_scripted_interaction() {
    let server_script = vec![
        ScriptedAction::ExpectClientMessage {
            matcher: ClientMessageMatcher::ByType(ClientMessageTypeMatcher::Register),
            response: Ok(None),
            timeout_duration: Some(Duration::from_secs(5)),
        },
        ScriptedAction::SendServerMessage(ServerMessage::AuctionStart(ServerAuctionStartMessage {
            intent_id: 1234567890123,
            intent: SwapIntent {
                src_chain: ApiIntentChain::Ethereum,
                dst_chain: ApiIntentChain::Solana,
                src_user: "0xTestSourceUser".to_string(),
                dst_user: "TestDestinationUserOnSolana".to_string(),
                token_in: "0xTokenAddressOnEthereum".to_string(),
                amount_in: "1000000000000000000".to_string(),
                token_out: "TokenAddressOnSolana".to_string(),
                amount_out: "990000000".to_string(),
                timeout: 300,
            },
        })),
        ScriptedAction::Delay(Duration::from_millis(500)),
        ScriptedAction::ExpectClientMessage {
            matcher: ClientMessageMatcher::ByType(ClientMessageTypeMatcher::Quote),
            response: Err(ServerErrorMessage {
                request_id: None,
                message: "ScriptedError".to_string(),
                code: 500, // HTTP equivalent
            }),
            timeout_duration: Some(Duration::from_secs(5)),
        },
        ScriptedAction::CloseConnectionGracefully,
    ];

    let mock_config = MockServerConfig {
        script_template: server_script,
    };

    let server = MockAuctioneer::new(mock_config)
        .await
        .expect("Failed to create mock server");
    let ws_url = server.ws_url();
    info!(
        "Mock server for test_simple_scripted_interaction listening at {}",
        ws_url
    );

    // Configure and connect the AuctioneerWsClient
    let client_config = AuctionClientConfig {
        max_reconnect_attempts: 0,
        connection_timeout: Duration::from_secs(3),
        ..Default::default()
    };

    let client = AuctioneerWsClient::connect(&ws_url, Some(client_config))
        .await
        .expect("Client failed to connect");

    client
        .wait_for_connection(Duration::from_secs(2))
        .await
        .expect("Client failed to establish connection in time");
    assert_eq!(client.connection_state().await, ConnectionState::Connected);

    let solver_id_str = "client_solver_01";
    let register_msg = ClientRegisterMessage::new(
        solver_id_str.to_string(),
        SolverAddresses {
            ethereum: Address::from_str("0x742d35Cc6634C0532925a3b844Bc454e4438f44e").unwrap(),
            solana: Pubkey::from_str("5zCZ3jk8EZnJyG7fhDqD6tmqiYTLZjik5HUpGMnHrZfC").unwrap(),
            base: Address::from_str("0x1111111111111111111111111111111111111111").unwrap(),
        },
    );
    client
        .register(register_msg)
        .await
        .expect("Failed to send register message");

    let auction_start_response = timeout(Duration::from_secs(2), client.receive_message())
        .await
        .expect("Timeout waiting for AuctionStart")
        .expect("Failed to receive AuctionStart");

    match auction_start_response {
        ServerMessage::AuctionStart(auction_start) => {
            assert_eq!(auction_start.intent_id, 1234567890123);
            assert_eq!(auction_start.intent.token_in, "0xTokenAddressOnEthereum");
        }
        _ => panic!(
            "Unexpected message: {:?}, expected AuctionStart",
            auction_start_response
        ),
    }

    let client_quote_request_id = Uuid::new_v4();
    let quote_msg = ClientQuoteMessage {
        request_id: client_quote_request_id,
        src_chain: "ethereum".to_string(),
        dst_chain: "solana".to_string(),
        solver_id: solver_id_str.to_string(),
        token_in: "0xTokenAddressOnEthereum".to_string(),
        amount_in: "1000000000000000000".to_string(),
        token_out: "TokenAddressOnSolana".to_string(),
        amount_out: "980000000".to_string(),
    };
    client
        .quote(quote_msg)
        .await
        .expect("Failed to send quote message");

    let client_quote_response = timeout(Duration::from_secs(2), client.receive_message())
        .await
        .expect("Timeout waiting for Quote acknowledgment")
        .expect("Failed to receive Quote acknowledgment");

    match client_quote_response {
        ServerMessage::Error(error_msg) => {
            assert_eq!(error_msg.code, 500);
            assert_eq!(error_msg.message, "ScriptedError");
            assert_eq!(
                error_msg.request_id,
                Some(client_quote_request_id),
                "Server did not echo the correct request_id for quote ack"
            );
        }
        _ => panic!(
            "Unexpected message: {:?}, expected Error (as Ack)",
            client_quote_response
        ),
    }

    info!("test_simple_scripted_interaction finished.");
}

#[tokio::test]
#[traced_test]
async fn test_server_drops_connection_abruptly() {
    let server_script = vec![
        ScriptedAction::ExpectClientMessage {
            matcher: ClientMessageMatcher::ByType(ClientMessageTypeMatcher::Register),
            response: Ok(None),
            timeout_duration: Some(Duration::from_secs(5)),
        },
        ScriptedAction::ServerDown,
    ];

    let mock_config = MockServerConfig {
        script_template: server_script,
    };

    let server = MockAuctioneer::new(mock_config)
        .await
        .expect("Failed to create mock server");
    let ws_url = server.ws_url();
    info!(
        "Mock server for test_server_drops_connection_abruptly listening at {}",
        ws_url
    );

    let client_config = AuctionClientConfig {
        max_reconnect_attempts: 2,
        reconnect_base_delay: Duration::from_millis(50),
        reconnect_max_delay: Duration::from_millis(200),
        connection_timeout: Duration::from_secs(3),
        ping_interval: Duration::from_secs(1),
        ..Default::default()
    };
    let client = AuctioneerWsClient::connect(&ws_url, Some(client_config))
        .await
        .expect("Client failed to connect initially");

    client
        .wait_for_connection(Duration::from_secs(4))
        .await
        .expect("Client failed to establish initial connection");
    assert_eq!(client.connection_state().await, ConnectionState::Connected);

    let register_msg = ClientRegisterMessage::new(
        "client_solver_drop_test".to_string(),
        SolverAddresses {
            ethereum: Address::from_str("0x2222222222222222222222222222222222222222").unwrap(),
            solana: Pubkey::from_str("5zCZ3jk8EZnJyG7fhDqD6tmqiYTLZjik5HUpGMnHrZfC").unwrap(),
            base: Address::from_str("0x3333333333333333333333333333333333333333").unwrap(),
        },
    );
    client
        .register(register_msg)
        .await
        .expect("Failed to send register message");

    let mut entered_reconnecting_or_disconnected = false;
    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let current_state = client.connection_state().await;
        info!("Client state: {:?}", current_state);
        if current_state == ConnectionState::Reconnecting || current_state == ConnectionState::Disconnected {
            info!("Client is {:?} as expected after drop.", current_state);
            entered_reconnecting_or_disconnected = true;
            break;
        }
    }
    assert!(
        entered_reconnecting_or_disconnected,
        "Client did not enter Reconnecting or Disconnected state after server went down."
    );

    info!("Waiting for client to reach Disconnected state after exhausting retries...");
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if client.connection_state().await == ConnectionState::Disconnected {
            break;
        }
    }
    assert_eq!(
        client.connection_state().await,
        ConnectionState::Disconnected,
        "Client should be Disconnected after retries against a dropping server."
    );

    info!("test_server_drops_connection_abruptly finished.");
}
