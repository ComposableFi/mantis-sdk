use auctioneer_api::http::{
    CheckHealthResponse, GetStatsResponse, GetSwapIntentResponse, GetTimeSeriesResponse, ListQuotesResponse,
    ListSwapIntentsResponse, StatsAsset, StatsSolver, SwapIntent,
};
use auctioneer_api::IntentChain;
use hyper::header::{HeaderValue, CONTENT_TYPE};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::sync::{Mutex, Notify};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointBehavior {
    Normal,
    Timeout,
    InternalServerError,
    Unauthorized,
    MalformedJson,
    SlowResponse(Duration),
}

#[derive(Debug, Clone)]
pub enum SequentialBehavior {
    Single(EndpointBehavior),
    Sequence(VecDeque<EndpointBehavior>),
}

impl SequentialBehavior {
    pub fn next_behavior(&mut self) -> EndpointBehavior {
        match self {
            Self::Single(behavior) => *behavior,
            Self::Sequence(behaviors) => {
                if behaviors.is_empty() {
                    EndpointBehavior::Normal
                } else {
                    behaviors.pop_front().unwrap_or(EndpointBehavior::Normal)
                }
            }
        }
    }
}

impl From<EndpointBehavior> for SequentialBehavior {
    fn from(behavior: EndpointBehavior) -> Self {
        Self::Single(behavior)
    }
}

impl From<Vec<EndpointBehavior>> for SequentialBehavior {
    fn from(behaviors: Vec<EndpointBehavior>) -> Self {
        Self::Sequence(VecDeque::from(behaviors))
    }
}

pub struct EndpointConfig {
    pub behavior: SequentialBehavior,
    pub path: String,
}

impl EndpointConfig {
    pub fn new(path: &str, behavior: EndpointBehavior) -> Self {
        Self {
            behavior: behavior.into(),
            path: path.to_string(),
        }
    }
    
    pub fn with_sequence(path: &str, behaviors: Vec<EndpointBehavior>) -> Self {
        Self {
            behavior: behaviors.into(),
            path: path.to_string(),
        }
    }
}

#[derive(Clone, Default)]
pub struct MockServerConfig {
    pub endpoints: Arc<Mutex<HashMap<String, EndpointConfig>>>,
    pub request_counter: Arc<AtomicU64>,
    pub global_delay: Arc<Mutex<Option<Duration>>>,
    pub global_delay_applied: Arc<AtomicBool>,
    pub unresponsive: Arc<AtomicBool>,
}

impl MockServerConfig {
    pub fn new() -> Self {
        Self {
            endpoints: Arc::new(Mutex::new(HashMap::new())),
            request_counter: Arc::new(AtomicU64::new(0)),
            global_delay: Arc::new(Mutex::new(None)),
            global_delay_applied: Arc::new(AtomicBool::new(false)),
            unresponsive: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn was_global_delay_applied(&self) -> bool {
        self.global_delay_applied.load(Ordering::SeqCst)
    }

    pub async fn add_endpoint(&self, config: EndpointConfig) {
        let mut endpoints = self.endpoints.lock().await;
        endpoints.insert(config.path.clone(), config);
    }

    pub async fn set_global_delay(&self, delay: Option<Duration>) {
        let mut global_delay = self.global_delay.lock().await;
        *global_delay = delay;
    }

    pub fn set_unresponsive(&self, value: bool) {
        tracing::info!("Setting unresponsive to {}", value);
        self.unresponsive.store(value, Ordering::SeqCst);
    }

    pub fn increment_request_counter(&self) -> u64 {
        self.request_counter.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn get_request_count(&self) -> u64 {
        self.request_counter.load(Ordering::SeqCst)
    }
}

pub struct MockHttpAuctioneer {
    addr: SocketAddr,
    config: MockServerConfig,
    shutdown_notifier: Arc<Notify>,
    is_shutting_down: Arc<AtomicBool>,
    _server_handle: Option<JoinHandle<Result<(), hyper::Error>>>,
}

impl Clone for MockHttpAuctioneer {
    fn clone(&self) -> Self {
        Self {
            addr: self.addr,
            config: self.config.clone(),
            shutdown_notifier: self.shutdown_notifier.clone(),
            is_shutting_down: self.is_shutting_down.clone(),
            _server_handle: None, // We don't clone the server handle, just the reference to the mock server
        }
    }
}

impl MockHttpAuctioneer {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let addr: SocketAddr = ([127, 0, 0, 1], 0).into();
        let config = MockServerConfig::new();
        let server_config = config.clone();
        let shutdown_notifier = Arc::new(Notify::new());
        let is_shutting_down = Arc::new(AtomicBool::new(false));

        let shutdown_signal = shutdown_notifier.clone();
        let is_shutting_down_clone = is_shutting_down.clone();

        let make_svc = make_service_fn(move |_conn| {
            let server_config = server_config.clone();
            let is_shutting_down = is_shutting_down_clone.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let server_config = server_config.clone();
                    let is_shutting_down = is_shutting_down.clone();
                    async move {
                        if is_shutting_down.load(Ordering::SeqCst) {
                            return Ok::<_, Infallible>(
                                Response::builder()
                                    .status(StatusCode::SERVICE_UNAVAILABLE)
                                    .body(Body::from("Server is shutting down"))
                                    .unwrap(),
                            );
                        }

                        let count = server_config.increment_request_counter();
                        tracing::info!("Incremented request counter to {}", count);

                        // Process the request
                        handle_request(req, server_config.clone()).await
                    }
                }))
            }
        });

        let server = Server::try_bind(&addr)?.http1_only(true).serve(make_svc);
        let server_addr = server.local_addr();

        let graceful = server.with_graceful_shutdown(async move {
            shutdown_signal.notified().await;
            info!("Shutdown signal received, HTTP mock server shutting down");
        });

        let server_handle = tokio::spawn(graceful);

        Ok(Self {
            addr: server_addr,
            config,
            shutdown_notifier,
            is_shutting_down,
            _server_handle: Some(server_handle),
        })
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn config(&self) -> MockServerConfig {
        self.config.clone()
    }
}

impl Drop for MockHttpAuctioneer {
    fn drop(&mut self) {
        if !self.is_shutting_down.load(Ordering::SeqCst) {
            self.is_shutting_down.store(true, Ordering::SeqCst);
            self.shutdown_notifier.notify_one();
        }
    }
}

async fn handle_request(req: Request<Body>, config: MockServerConfig) -> Result<Response<Body>, Infallible> {
    // Check if server is globally unresponsive
    if config.unresponsive.load(Ordering::SeqCst) {
        tracing::info!("Server is unresponsive, will sleep for 10 seconds");
        // Simulate an unresponsive server by sleeping for a long time
        sleep(Duration::from_secs(10)).await;
        return Ok(Response::new(Body::empty()));
    }

    // Apply global delay if any - do this before any other processing
    {
        // Use a separate scope for the mutex lock
        let global_delay_guard = config.global_delay.lock().await;
        if let Some(delay) = *global_delay_guard {
            tracing::info!("Applying global delay of {:?}", delay);
            drop(global_delay_guard); // Drop the lock before sleeping

            // Set the flag first so tests can verify the delay was attempted
            config.global_delay_applied.store(true, Ordering::SeqCst);

            // Actually sleep for the delay
            sleep(delay).await;

            tracing::info!("Global delay completed");
        }
    }

    let path = req.uri().path().to_string();
    let mut endpoints = config.endpoints.lock().await;

    // Check if we have a specific behavior for this endpoint
    if let Some(endpoint_config) = endpoints.get_mut(&path) {
        let behavior = endpoint_config.behavior.next_behavior();
        match behavior {
            EndpointBehavior::Normal => {
                // Generate a normal response based on the path
                let response = generate_mock_response(&path);
                Ok(response)
            }
            EndpointBehavior::Timeout => {
                // Simulate a timeout by sleeping for longer than client timeout
                sleep(Duration::from_secs(10)).await;
                Ok(Response::new(Body::empty()))
            }
            EndpointBehavior::InternalServerError => 
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("Internal Server Error"))
                    .unwrap()),
            EndpointBehavior::Unauthorized => 
                Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Body::from("Unauthorized"))
                    .unwrap()),
            EndpointBehavior::MalformedJson => Ok(Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .body(Body::from("{invalid json:}"))
                .unwrap()),
            EndpointBehavior::SlowResponse(delay) => {
                sleep(delay).await;
                let response = generate_mock_response(&path);
                Ok(response)
            }
        }
    } else {
        // Default behavior for endpoints without specific configuration
        let response = generate_mock_response(&path);
        Ok(response)
    }
}

fn generate_mock_response(path: &str) -> Response<Body> {
    // Default responses based on endpoint path
    match path {
        "/api/v1/health" => json_response(&CheckHealthResponse {
            status: "ok".to_string(),
        }),
        "/api/v1/intents" => {
            let mock_intent = create_mock_intent(123);
            json_response(&ListSwapIntentsResponse {
                page: 1,
                items: 1,
                page_size: 10,
                page_max: 1,
                items_max: 1,
                intents: vec![mock_intent],
            })
        }
        "/api/v1/quotes" => json_response(&ListQuotesResponse {
            src_chain: "ethereum".to_string(),
            dst_chain: "solana".to_string(),
            solver_quotes: vec![],
        }),
        "/api/v1/stats" => json_response(&GetStatsResponse {
            total_trades: 10,
            unique_addresses: 5,
            total_volume: 1000.0,
            total_local_volume: 500.0,
            total_remote_volume: 500.0,
            total_value_in: 1000.0,
            total_value_out: 990.0,
            total_fees: 10.0,
            top_assets: vec![StatsAsset {
                address: "0xTokenAddress".to_string(),
                symbol: Some("ETH".to_string()),
                volume: 500.0,
            }],
            top_solvers: vec![StatsSolver {
                address: "0xSolverAddress".to_string(),
                volume: 1000.0,
            }],
        }),
        "/api/v1/time_series" => json_response(&GetTimeSeriesResponse {
            start_timestamp: 1609459200, // 2021-01-01
            end_timestamp: 1609545600,   // 2021-01-02
            total_trades: vec![10],
            total_volume: vec![1000.0],
            total_value_in: vec![1000.0],
            total_value_out: vec![990.0],
            total_fees: vec![10.0],
        }),
        "/api/v1/fees" => json_response(&auctioneer_api::http::ListFeesResponse {
            solana: vec![],
            ethereum: vec![],
        }),
        _ if path.starts_with("/api/v1/intents/") => {
            let id_str = path.trim_start_matches("/api/v1/intents/");
            let id = id_str.parse::<u64>().unwrap_or(123);
            let mock_intent = create_mock_intent(id);
            json_response(&GetSwapIntentResponse { intent: mock_intent })
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap(),
    }
}

fn create_mock_intent(id: u64) -> SwapIntent {
    SwapIntent {
        intent_id: id,
        created_at: "2023-01-01T00:00:00Z".to_string(),
        escrow_transaction: format!("0x{}", "0".repeat(64)),
        src_user: "0xTestUser".to_string(),
        dst_user: "TestSolanaUser".to_string(),
        src_chain: IntentChain::Ethereum.to_string().to_lowercase(),
        dst_chain: IntentChain::Solana.to_string().to_lowercase(),
        token_in: "0xTestToken".to_string(),
        amount_in: "1000000000000000000".to_string(),
        token_out: "TestSolanaToken".to_string(),
        amount_wanted: "1000000000".to_string(),
        amount_provided: None,
        fee_amount: "10000000000000000".to_string(), // 0.01 ETH
        timeout_sec: 300,
        is_canceled: false,
        is_solved: false,
        ai_agent: false,
        solver: None,
        canceled_at: None,
        solved_at: None,
        solve_transaction: None,
        token_in_price_usd: Some(2000.0),
        token_out_price_usd: Some(20.0),
        token_in_symbol: Some("ETH".to_string()),
        token_out_symbol: Some("SOL".to_string()),
        token_in_decimals: Some(18),
        token_out_decimals: Some(9),
    }
}

fn json_response<T: Serialize>(data: &T) -> Response<Body> {
    match serde_json::to_string(data) {
        Ok(json) => Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .body(Body::from(json))
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("Failed to serialize response"))
            .unwrap(),
    }
}
