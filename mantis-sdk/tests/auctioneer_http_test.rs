use auctioneer_api::http::{ListQuotesQuery, ListSwapIntentsQuery};
use mantis_sdk::auction::http::{AuctioneerHttpClient, HttpClientConfig};
use mantis_sdk::auction::IntentChain;
use num::BigUint;
use std::{env, sync::atomic::Ordering, time::Duration};
use tracing_test::traced_test;

mod mock_http_auctioneer;
use mock_http_auctioneer::{EndpointBehavior, EndpointConfig, MockHttpAuctioneer};

#[tokio::test]
#[traced_test]
async fn test_auth_failure_when_env_not_set() {
    let env_var_name = "UNIQUE_TEST_MANTIS_ADMIN_KEY_FOR_FAILURE_TEST";

    env::remove_var(env_var_name);

    // Verify that it was actually removed
    assert!(
        env::var(env_var_name).is_err(),
        "Environment variable {} should not be set",
        env_var_name
    );

    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    // Create the HTTP client with the unique env var name
    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(5),
            max_retries: 0,
            authority_env_var: env_var_name.to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    // Test that auth fails when env var is not set
    let result = client.list_fees().await;
    println!("Auth result: {:?}", result);

    assert!(result.is_err(), "Expected auth failure but got success");

    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("Authority key not found"),
        "Expected 'Authority key not found' error but got: {}",
        error
    );
}

#[tokio::test]
#[traced_test]
async fn test_auth_key_included_when_env_set() {
    env::remove_var("TEST_MANTIS_ADMIN_KEY");

    let key_value = "test_auth_key_value";

    env::set_var("TEST_MANTIS_ADMIN_KEY", key_value);

    let auth_key_result = env::var("TEST_MANTIS_ADMIN_KEY");
    assert!(auth_key_result.is_ok());
    assert_eq!(auth_key_result.unwrap(), key_value);

    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(5),
            max_retries: 0,
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    // Test auth succeeds when env var is set
    let result = client.list_fees().await;

    println!("Auth with env var set result: {:?}", result);
    assert!(result.is_ok(), "Expected auth to succeed when env var is set");

    env::remove_var("TEST_MANTIS_ADMIN_KEY");
}

#[tokio::test]
#[traced_test]
async fn test_http_client_with_successful_response() {
    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    let server_config = mock_server.config();
    server_config
        .add_endpoint(EndpointConfig::new("/api/v1/intents", EndpointBehavior::Normal))
        .await;

    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(5),
            max_retries: 0,
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    let query = ListSwapIntentsQuery {
        src_chain: None,
        period: None,
        src_user: vec![],
        page: None,
        page_size: None,
    };

    let result = client.list_swap_intents(query).await;
    assert!(
        result.is_ok(),
        "Expected successful response, but got error: {:?}",
        result.err()
    );

    let response = result.unwrap();
    assert_eq!(response.intents.len(), 1);
    assert_eq!(response.intents[0].intent_id, 123);
}

#[tokio::test]
#[traced_test]
async fn test_http_client_with_slow_response() {
    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    let server_config = mock_server.config();
    server_config
        .add_endpoint(EndpointConfig::new(
            "/api/v1/intents",
            EndpointBehavior::SlowResponse(Duration::from_millis(300)),
        ))
        .await;

    // Create the HTTP client with a longer timeout
    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(5),
            max_retries: 0,
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    let start = std::time::Instant::now();

    let query = ListSwapIntentsQuery {
        src_chain: None,
        period: None,
        src_user: vec![],
        page: None,
        page_size: None,
    };

    let result = client.list_swap_intents(query).await;

    // Verify timing
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() >= 300,
        "Expected response to take at least 300ms"
    );

    assert!(
        result.is_ok(),
        "Expected successful response despite slowness, but got error: {:?}",
        result.err()
    );
}

#[tokio::test]
#[traced_test]
async fn test_http_client_with_timeout() {
    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    // Configure the mock server with a timeout endpoint
    let server_config = mock_server.config();
    server_config
        .add_endpoint(EndpointConfig::new("/api/v1/intents", EndpointBehavior::Timeout))
        .await;

    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(2), // Shorter than the mock timeout
            max_retries: 0,
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    let query = ListSwapIntentsQuery {
        src_chain: None,
        period: None,
        src_user: vec![],
        page: None,
        page_size: None,
    };

    let result = client.list_swap_intents(query).await;
    assert!(result.is_err(), "Expected timeout error");
    assert!(
        result.unwrap_err().to_string().contains("timed out"),
        "Expected timeout error message"
    );
}

#[tokio::test]
#[traced_test]
async fn test_http_client_with_unauthorized_response() {
    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    // Configure the mock server with an unauthorized endpoint
    let server_config = mock_server.config();
    server_config
        .add_endpoint(EndpointConfig::new(
            "/api/v1/intents",
            EndpointBehavior::Unauthorized,
        ))
        .await;

    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(5),
            max_retries: 0,
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    let query = ListSwapIntentsQuery {
        src_chain: None,
        period: None,
        src_user: vec![],
        page: None,
        page_size: None,
    };

    let result = client.list_swap_intents(query).await;
    assert!(result.is_err(), "Expected unauthorized error");
    assert!(
        result.unwrap_err().to_string().contains("Unauthorized"),
        "Expected unauthorized error message"
    );
}

#[tokio::test]
#[traced_test]
async fn test_http_client_with_server_error() {
    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    // Configure the mock server with a server error endpoint
    let server_config = mock_server.config();
    server_config
        .add_endpoint(EndpointConfig::new(
            "/api/v1/intents",
            EndpointBehavior::InternalServerError,
        ))
        .await;

    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(5),
            max_retries: 0, // No retries to make test faster
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    let query = ListSwapIntentsQuery {
        src_chain: None,
        period: None,
        src_user: vec![],
        page: None,
        page_size: None,
    };

    let result = client.list_swap_intents(query).await;
    assert!(result.is_err(), "Expected server error");
    assert!(
        result.unwrap_err().to_string().contains("HTTP 500"),
        "Expected 500 error message"
    );
}

#[tokio::test]
#[traced_test]
async fn test_http_client_with_malformed_json_response() {
    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    // Configure the mock server with a malformed JSON response
    let server_config = mock_server.config();
    server_config
        .add_endpoint(EndpointConfig::new(
            "/api/v1/intents",
            EndpointBehavior::MalformedJson,
        ))
        .await;

    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(5),
            max_retries: 0,
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    let query = ListSwapIntentsQuery {
        src_chain: None,
        period: None,
        src_user: vec![],
        page: None,
        page_size: None,
    };

    let result = client.list_swap_intents(query).await;
    assert!(result.is_err(), "Expected parsing error");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse response"),
        "Expected parsing error message"
    );
}

#[tokio::test]
#[traced_test]
async fn test_http_client_with_retry_success() {
    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    // Reset the request counter
    let server_config = mock_server.config();
    server_config.request_counter.store(0, Ordering::SeqCst);

    let behaviors = vec![EndpointBehavior::InternalServerError, EndpointBehavior::Normal];

    server_config
        .add_endpoint(EndpointConfig::with_sequence("/api/v1/intents", behaviors))
        .await;

    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(5),
            max_retries: 3,
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    let query = ListSwapIntentsQuery {
        src_chain: None,
        period: None,
        src_user: vec![],
        page: None,
        page_size: None,
    };

    let result = client.list_swap_intents(query).await;

    // Sleep a bit to make sure all requests are counted
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Get the request count after the test
    let request_count = server_config.get_request_count();
    println!("Final request count: {}", request_count);

    // Assertions
    assert!(result.is_ok(), "Expected success after retry");
    assert!(
        request_count > 1,
        "Expected multiple requests due to retry, but got {}",
        request_count
    );
}

#[tokio::test]
#[traced_test]
async fn test_http_client_with_global_delay() {
    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    let server_config = mock_server.config();
    server_config
        .add_endpoint(EndpointConfig::new("/api/v1/intents", EndpointBehavior::Normal))
        .await;

    // Force a long delay that we can easily detect - 1 second
    let delay_duration = Duration::from_secs(1);
    println!("Setting global delay to {:?}", delay_duration);
    server_config.set_global_delay(Some(delay_duration)).await;

    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(5),
            max_retries: 0,
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    assert!(
        !server_config.was_global_delay_applied(),
        "Global delay should not have been applied yet"
    );

    // Make a request to list intents
    let query = ListSwapIntentsQuery {
        src_chain: None,
        period: None,
        src_user: vec![],
        page: None,
        page_size: None,
    };

    let result = client.list_swap_intents(query).await;

    assert!(
        server_config.was_global_delay_applied(),
        "Global delay should have been applied"
    );

    assert!(result.is_ok(), "Expected successful response despite delay");
}

#[tokio::test]
#[traced_test]
async fn test_http_client_with_unresponsive_server() {
    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    // Configure the mock server to be globally unresponsive
    let server_config = mock_server.config();
    server_config.set_unresponsive(true);

    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_millis(500), // Short timeout for test
            max_retries: 0,
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    let query = ListSwapIntentsQuery {
        src_chain: None,
        period: None,
        src_user: vec![],
        page: None,
        page_size: None,
    };

    let result = client.list_swap_intents(query).await;
    assert!(result.is_err(), "Expected timeout due to unresponsive server");
    assert!(
        result.unwrap_err().to_string().contains("timed out"),
        "Expected timeout error message"
    );
}

#[tokio::test]
#[traced_test]
async fn test_http_client_get_swap_intent() {
    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    let server_config = mock_server.config();
    server_config
        .add_endpoint(EndpointConfig::new(
            "/api/v1/intents/123",
            EndpointBehavior::Normal,
        ))
        .await;

    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(5),
            max_retries: 0,
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    let result = client.get_swap_intent(123).await;

    assert!(result.is_ok(), "Expected successful response");
    let intent_response = result.unwrap();
    assert_eq!(intent_response.intent.intent_id, 123);
    assert_eq!(intent_response.intent.src_chain.to_lowercase(), "ethereum");
    assert_eq!(intent_response.intent.dst_chain.to_lowercase(), "solana");
}

#[tokio::test]
#[traced_test]
async fn test_quotes_endpoint() {
    let mock_server = MockHttpAuctioneer::new().await.unwrap();

    let server_config = mock_server.config();
    server_config
        .add_endpoint(EndpointConfig::new("/api/v1/quotes", EndpointBehavior::Normal))
        .await;

    let client = AuctioneerHttpClient::new(
        &mock_server.base_url(),
        Some(HttpClientConfig {
            request_timeout: Duration::from_secs(5),
            max_retries: 0,
            authority_env_var: "TEST_MANTIS_ADMIN_KEY".to_string(),
            retry_base_delay: Duration::from_millis(100),
            api_version: "v1".to_string(),
        }),
    )
    .unwrap();

    let query = ListQuotesQuery {
        src_chain: IntentChain::Ethereum,
        dst_chain: IntentChain::Solana,
        token_in: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
        token_in_amount: BigUint::from(1000000000u64),
        token_out: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
    };

    let result = client.list_quotes(query).await;
    assert!(result.is_ok(), "Expected successful quote response");

    let quotes = result.unwrap();
    assert_eq!(quotes.src_chain.to_lowercase(), "ethereum");
    assert_eq!(quotes.dst_chain.to_lowercase(), "solana");
}
