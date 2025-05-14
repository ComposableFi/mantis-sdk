use anyhow::{anyhow, Context, Result};
use auctioneer_api::ws::{ClientMessage, ClientRegisterMessage, ServerMessage};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use url::Url;

/// Connection state of the WebSocket client
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Client is connected to the server
    Connected,
    /// Client is disconnected and trying to reconnect
    Reconnecting,
    /// Client is disconnected and not trying to reconnect
    Disconnected,
    /// Client is shutting down
    ShuttingDown,
}

/// Configuration for the WebSocket client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Maximum number of reconnection attempts (0 for indefinite)
    pub max_reconnect_attempts: u32,
    /// Base delay between reconnection attempts (grows exponentially)
    pub reconnect_base_delay: Duration,
    /// Maximum delay between reconnection attempts
    pub reconnect_max_delay: Duration,
    /// Size of the message channels
    pub channel_size: usize,
    /// Ping interval to keep the connection alive
    pub ping_interval: Duration,
    /// Timeout for connection attempts
    pub connection_timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            max_reconnect_attempts: 5, // Set to 0 for indefinite retries
            reconnect_base_delay: Duration::from_secs(1),
            reconnect_max_delay: Duration::from_secs(60),
            channel_size: 100,
            ping_interval: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(10),
        }
    }
}

type WsWriter = Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>;

/// WebSocket client for communicating with the auctioneer service
/// with support for automatic reconnection and error handling
pub struct AuctioneerWsClient {
    /// URL of the auctioneer WebSocket service
    url: Url,
    /// Sender half of the channel used to send messages to the connection_manager task
    tx_to_manager: mpsc::Sender<ClientMessage>,
    /// Channel receiver for incoming server messages from the connection_manager task
    rx_from_manager: Arc<Mutex<mpsc::Receiver<ServerMessage>>>,
    /// Client configuration
    config: ClientConfig,
    /// Current connection state
    state: Arc<RwLock<ConnectionState>>,
    /// Outgoing message buffer that couldn't be sent due to disconnection
    outgoing_buffer: Arc<Mutex<Vec<ClientMessage>>>,
    /// Most recent registration message for reconnection
    last_registration: Arc<Mutex<Option<ClientRegisterMessage>>>,
    /// Token to signal shutdown to the connection_manager task
    shutdown_token: CancellationToken,
}

impl AuctioneerWsClient {
    pub async fn connect(url_str: &str, config: Option<ClientConfig>) -> Result<Self> {
        let url = Url::parse(url_str).context("Invalid WebSocket URL")?;
        let config = config.unwrap_or_default();

        let (api_tx, manager_rx_client_msg) = mpsc::channel::<ClientMessage>(config.channel_size);
        let (manager_tx_server_msg, api_rx) = mpsc::channel::<ServerMessage>(config.channel_size);

        let state = Arc::new(RwLock::new(ConnectionState::Disconnected));
        let outgoing_buffer = Arc::new(Mutex::new(Vec::new()));
        let last_registration = Arc::new(Mutex::new(None));
        let shutdown_token = CancellationToken::new();

        let manager_url = url.clone();
        let manager_config = config.clone();
        let manager_state = state.clone();
        let manager_outgoing_buffer = outgoing_buffer.clone();
        let manager_last_registration = last_registration.clone();
        let manager_shutdown_token = shutdown_token.clone();

        // Spawn the connection manager task
        tokio::spawn(async move {
            connection_manager(
                manager_url,
                manager_config,
                manager_state,
                manager_rx_client_msg,
                manager_tx_server_msg,
                manager_outgoing_buffer,
                manager_last_registration,
                manager_shutdown_token,
            )
            .await;
        });

        Ok(Self {
            url,
            tx_to_manager: api_tx,
            rx_from_manager: Arc::new(Mutex::new(api_rx)),
            config,
            state,
            outgoing_buffer,
            last_registration,
            shutdown_token,
        })
    }

    /// Send a client message to the auctioneer service
    pub async fn send_message(&self, message: ClientMessage) -> Result<()> {
        if let ClientMessage::Register(ref reg_msg) = message {
            *self.last_registration.lock().await = Some(reg_msg.clone());
        }

        let current_state = *self.state.read().await;
        match current_state {
            ConnectionState::Connected => {
                self.tx_to_manager
                    .send(message)
                    .await
                    .map_err(|_| anyhow!("Failed to send message: Connection manager task closed"))?;
            }
            ConnectionState::Reconnecting | ConnectionState::Disconnected => {
                debug!("Connection not ready ({:?}), buffering message", current_state);
                self.outgoing_buffer.lock().await.push(message);
                if current_state == ConnectionState::Disconnected {
                    // Optionally, trigger a reconnect attempt if fully disconnected and not already trying
                    // This depends on desired behavior; currently manager task handles retries.
                    warn!("Message buffered while client is fully disconnected. It will be sent upon reconnection.");
                }
            }
            ConnectionState::ShuttingDown => {
                return Err(anyhow!("Client is shutting down, cannot send message."));
            }
        }
        Ok(())
    }

    /// Receive a server message from the auctioneer service
    pub async fn receive_message(&self) -> Result<ServerMessage> {
        let mut rx_guard = self.rx_from_manager.lock().await;
        rx_guard
            .recv()
            .await
            .ok_or_else(|| anyhow!("WebSocket connection closed and manager task terminated"))
    }

    /// Try to receive a server message from the auctioneer service
    pub async fn try_receive_message(&self) -> Result<Option<ServerMessage>> {
        let mut rx_guard = self.rx_from_manager.lock().await;
        match rx_guard.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(mpsc::error::TryRecvError::Empty) => Ok(None),
            Err(mpsc::error::TryRecvError::Disconnected) => {
                Err(anyhow!("WebSocket connection closed and manager task terminated"))
            }
        }
    }

    // Helper methods
    pub async fn register(&self, register_message: ClientRegisterMessage) -> Result<()> {
        self.send_message(ClientMessage::Register(register_message)).await
    }
    pub async fn bid(&self, bid_message: auctioneer_api::ws::ClientBidMessage) -> Result<()> {
        self.send_message(ClientMessage::Bid(bid_message)).await
    }
    pub async fn solve(&self, solve_message: auctioneer_api::ws::ClientSolveMessage) -> Result<()> {
        self.send_message(ClientMessage::Solve(solve_message)).await
    }
    pub async fn quote(&self, quote_message: auctioneer_api::ws::ClientQuoteMessage) -> Result<()> {
        self.send_message(ClientMessage::Quote(quote_message)).await
    }

    pub async fn connection_state(&self) -> ConnectionState {
        *self.state.read().await
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    pub async fn wait_for_connection(&self, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        loop {
            if *self.state.read().await == ConnectionState::Connected {
                return Ok(());
            }
            if start.elapsed() >= timeout {
                return Err(anyhow!("Timed out waiting for connection"));
            }
            if self.shutdown_token.is_cancelled() {
                return Err(anyhow!("Client is shutting down while waiting for connection"));
            }
            sleep(Duration::from_millis(100)).await;
        }
    }
}

impl Drop for AuctioneerWsClient {
    fn drop(&mut self) {
        info!("AuctioneerWsClient is being dropped, signalling shutdown.");
        self.shutdown_token.cancel();
        // The connection_manager task will handle graceful shutdown of the WebSocket.
    }
}

async fn connection_manager(
    url: Url,
    config: ClientConfig,
    state: Arc<RwLock<ConnectionState>>,
    mut client_msg_rx: mpsc::Receiver<ClientMessage>, // Receives messages from API via self.tx_to_manager
    server_msg_tx: mpsc::Sender<ServerMessage>,       // Sends messages to API via self.rx_from_manager
    outgoing_buffer: Arc<Mutex<Vec<ClientMessage>>>,
    last_registration: Arc<Mutex<Option<ClientRegisterMessage>>>,
    shutdown_token: CancellationToken,
) {
    let mut attempts = 0;

    'reconnect_loop: loop {
        if shutdown_token.is_cancelled() {
            info!("Shutdown signalled, connection manager exiting.");
            *state.write().await = ConnectionState::ShuttingDown;
            break;
        }

        if attempts > 0 {
            // Max attempts check (0 means infinite)
            if config.max_reconnect_attempts > 0 && attempts >= config.max_reconnect_attempts {
                error!(
                    "Max reconnection attempts ({}) reached. Giving up.",
                    config.max_reconnect_attempts
                );
                *state.write().await = ConnectionState::Disconnected;
                break; // Exit manager task
            }

            let delay_pow = attempts.saturating_sub(1); // First retry (attempts=1) has delay_pow=0
            let delay = std::cmp::min(
                config.reconnect_base_delay * 2u32.pow(delay_pow),
                config.reconnect_max_delay,
            );
            warn!(
                "Attempting to reconnect in {:?} (attempt {}/{})",
                delay,
                attempts,
                if config.max_reconnect_attempts == 0 {
                    "infinite".to_string()
                } else {
                    config.max_reconnect_attempts.to_string()
                }
            );

            tokio::select! {
                _ = sleep(delay) => {},
                _ = shutdown_token.cancelled() => {
                    info!("Shutdown signalled during backoff, connection manager exiting.");
                    *state.write().await = ConnectionState::ShuttingDown;
                    return;
                }
            }
        }

        *state.write().await = ConnectionState::Reconnecting;
        info!("Attempting to connect to {}...", url);

        let connect_future = connect_async(&url);
        let ws_stream_result = tokio::time::timeout(config.connection_timeout, connect_future).await;

        let ws_stream = match ws_stream_result {
            Ok(Ok((stream, _response))) => {
                info!("Successfully connected to {}", url);
                *state.write().await = ConnectionState::Connected;
                attempts = 0; // Reset attempts on successful connection
                stream
            }
            Ok(Err(err)) => {
                error!("Failed to connect to {}: {}", url, err);
                attempts += 1;
                continue; // Retry connection
            }
            Err(_) => {
                error!(
                    "Connection to {} timed out after {:?}",
                    url, config.connection_timeout
                );
                attempts += 1;
                continue; // Retry connection
            }
        };

        let (ws_writer_raw, mut ws_reader) = ws_stream.split();
        let ws_writer: WsWriter = Arc::new(Mutex::new(ws_writer_raw));

        // Send buffered messages
        {
            let mut buffer_guard = outgoing_buffer.lock().await;
            if !buffer_guard.is_empty() {
                info!("Sending {} buffered messages...", buffer_guard.len());
                // Drain messages into a temporary vector to release the lock sooner.
                let mut messages_to_process: VecDeque<_> = buffer_guard.drain(..).collect();
                drop(buffer_guard);

                while let Some(msg) = messages_to_process.pop_front() {
                    // Process one by one
                    if shutdown_token.is_cancelled() {
                        // If shutdown, put this message and the rest back into the shared buffer
                        let mut re_buffer_guard = outgoing_buffer.lock().await;
                        re_buffer_guard.push(msg);
                        re_buffer_guard.extend(messages_to_process); // The rest
                        info!("Shutdown during buffered send. Re-buffered remaining messages.");
                        break; // Stop processing
                    }

                    match serde_json::to_string(&msg) {
                        Ok(json_msg) => {
                            let mut writer_guard = ws_writer.lock().await;
                            if let Err(e) = writer_guard.send(Message::Text(json_msg)).await {
                                error!("Failed to send buffered message: {}. Re-buffering this and subsequent messages.", e);

                                // Put the failed message and the rest of messages_to_process back
                                {
                                    let mut re_buffer_guard = outgoing_buffer.lock().await;
                                    re_buffer_guard.push(msg);
                                    re_buffer_guard.extend(messages_to_process);
                                }

                                *state.write().await = ConnectionState::Reconnecting;
                                attempts += 1;
                                if let Err(e) =
                                    tokio::time::timeout(Duration::from_secs(1), writer_guard.close()).await
                                {
                                    error!("Timeout while closing WebSocket writer: {}", e);
                                }
                                break; // Stop processing, will trigger reconnect from outer loop
                            }
                        }
                        Err(e) => {
                            warn!("Failed to serialize buffered message: {}. Message dropped.", e);
                            // This specific message (msg) is lost. The loop continues with the next from messages_to_process.
                        }
                    }
                }

                if *state.read().await == ConnectionState::Reconnecting {
                    continue; // If sending buffered failed, reconnect
                }
            }
        }

        // Check after potentially long buffered send
        if shutdown_token.is_cancelled() {
            continue;
        }

        // Re-register if needed
        if let Some(reg_msg) = last_registration.lock().await.clone() {
            info!("Re-registering with server...");
            match serde_json::to_string(&ClientMessage::Register(reg_msg)) {
                Ok(json_msg) => {
                    if let Err(e) = ws_writer.lock().await.send(Message::Text(json_msg)).await {
                        error!(
                            "Failed to send re-registration message: {}. Will retry on next connection.",
                            e
                        );
                        *state.write().await = ConnectionState::Reconnecting;
                        attempts += 1;
                        continue; // Trigger reconnect
                    }
                }
                Err(e) => error!("Failed to serialize re-registration message: {}", e),
            }
        }

        if shutdown_token.is_cancelled() {
            continue;
        }

        let mut ping_ticker = tokio::time::interval(config.ping_interval);

        // Main message loop for this connection
        loop {
            tokio::select! {
                _ = shutdown_token.cancelled() => {
                    info!("Shutdown signalled. Closing WebSocket connection.");
                    *state.write().await = ConnectionState::ShuttingDown;
                    let _ = ws_writer.lock().await.send(Message::Close(None)).await; // Try to send Close frame
                    let _ = ws_writer.lock().await.close().await;
                    break 'reconnect_loop; // Exit manager task completely
                }

                Some(client_msg) = client_msg_rx.recv() => {
                    match serde_json::to_string(&client_msg) {
                        Ok(json_msg) => {
                            if let Err(e) = ws_writer.lock().await.send(Message::Text(json_msg)).await {
                                error!("Failed to send client message: {}. Buffering and attempting reconnect.", e);
                                outgoing_buffer.lock().await.push(client_msg);
                                *state.write().await = ConnectionState::Reconnecting;
                                attempts +=1;
                                break; // Break select loop, trigger reconnect
                            }
                        }
                        Err(e) => {
                            error!("Failed to serialize client message: {}", e);
                        }
                    }
                }

                Some(ws_result) = ws_reader.next() => {
                    match ws_result {
                        Ok(ws_msg) => match ws_msg {
                            Message::Text(text) => {
                                match serde_json::from_str::<ServerMessage>(&text) {
                                    Ok(server_msg) => {
                                        if server_msg_tx.send(server_msg).await.is_err() {
                                            error!("Failed to forward server message to API: API receiver dropped.");
                                            // This implies the client struct was dropped or API rx closed.
                                            // We can consider this a form of shutdown.
                                            *state.write().await = ConnectionState::ShuttingDown;
                                             let _ = ws_writer.lock().await.close().await;
                                            return; // Exit manager task
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to deserialize server message: {}", e);
                                    }
                                }
                            }
                            Message::Binary(_) => {
                                debug!("Received binary message, ignoring.");
                            }
                            Message::Ping(data) => {
                                debug!("Received Ping from server, sending Pong.");
                                if let Err(e) = ws_writer.lock().await.send(Message::Pong(data)).await {
                                    error!("Failed to send Pong: {}. Attempting reconnect.", e);
                                    *state.write().await = ConnectionState::Reconnecting;
                                    attempts +=1;
                                    break; // Break select loop, trigger reconnect
                                }
                            }
                            Message::Pong(_) => {
                                debug!("Received Pong from server.");
                            }
                            Message::Close(close_frame) => {
                                info!("WebSocket connection closed by server: {:?}", close_frame);
                                *state.write().await = ConnectionState::Reconnecting;
                                attempts +=1;
                                break; // Break select loop, trigger reconnect
                            }
                            Message::Frame(_) => { /* Ignore raw frames */ }
                        },
                        Err(e) => { // WebSocket read error
                            error!("WebSocket read error: {}. Attempting reconnect.", e);
                            *state.write().await = ConnectionState::Reconnecting;
                            attempts +=1;
                            break; // Break select loop, trigger reconnect
                        }
                    }
                }

                _ = ping_ticker.tick() => {
                    if *state.read().await == ConnectionState::Connected {
                         debug!("Sending WebSocket Ping to keep connection alive.");
                        if let Err(e) = ws_writer.lock().await.send(Message::Ping(Vec::new())).await {
                            error!("Failed to send Ping: {}. Attempting reconnect.", e);
                            *state.write().await = ConnectionState::Reconnecting;
                            attempts +=1;
                            break; // Break select loop, trigger reconnect
                        }
                    }
                }

                else => { // One of the channels closed unexpectedly
                    info!("A channel closed unexpectedly. Assuming disconnection.");
                    *state.write().await = ConnectionState::Reconnecting;
                    attempts +=1;
                    break;
                }
            }
        }

        // If we break from the select loop due to an error, the outer loop will handle reconnection.
        // If shutdown_token caused exit, the manager task returns.
        // Close the current connection sink before retrying
        let _ = ws_writer.lock().await.close().await;
    }

    // Manager task is ending, set final state if not already ShuttingDown
    let mut final_state_guard = state.write().await;
    if *final_state_guard != ConnectionState::ShuttingDown {
        *final_state_guard = ConnectionState::Disconnected;
    }
    info!("Connection manager has shut down.");
}
