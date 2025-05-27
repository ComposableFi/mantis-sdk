use auctioneer_api::ws::{ClientMessage, ServerErrorMessage, ServerMessage};
use futures::{SinkExt, StreamExt};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex, Notify};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout, Duration};
use tokio_tungstenite::{accept_async, tungstenite::protocol::Message as WsMessage};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum ClientMessageTypeMatcher {
    Register,
    Bid,
    Solve,
    Quote,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ClientMessageMatcher {
    /// Matches any client message.
    Any,
    /// Matches a specific type of client message.
    ByType(ClientMessageTypeMatcher),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ScriptedAction {
    /// Expect a client message that matches the `matcher`.
    /// If a `response` is provided, send it back.
    ExpectClientMessage {
        matcher: ClientMessageMatcher,
        response: Result<Option<ServerMessage>, ServerErrorMessage>,
        timeout_duration: Option<Duration>,
    },
    /// Proactively send a `ServerMessage` to the client.
    SendServerMessage(ServerMessage),
    /// Introduce a delay in processing the script.
    Delay(Duration),
    /// Gracefully close the WebSocket connection from the server side (sends a Close frame).
    CloseConnectionGracefully,
    /// Abruptly terminate the connection (e.g., by closing the underlying TCP stream without a WebSocket Close frame).
    DropConnectionAbruptly,
    /// Server stops responding to any further client messages and does not process further script actions.
    /// The connection remains open until the client times out or closes it.
    BecomeUnresponsive,
    /// Server down, connection drops and server goes offline. This will shut down the entire MockAuctioneer.
    ServerDown,
}

#[derive(Clone, Debug, Default)]
pub struct MockServerConfig {
    /// A script of actions that each new client connection will follow.
    pub script_template: Vec<ScriptedAction>,
}

/// Mock server context for a single client connection.
struct ClientContext {
    /// Channel to send WebSocket messages (like Text or Close)
    sender_to_client_ws: mpsc::Sender<WsMessage>,
    /// This client's queue of actions to perform.
    action_queue: VecDeque<ScriptedAction>,
    /// Stores the client_id after successful registration, if needed by subsequent scripted responses.
    client_id: Option<String>,
    /// Stores the last received request_id, if needed by scripted responses.
    last_request_id: Option<Uuid>,
}

impl ClientContext {
    fn new(sender_to_client_ws: mpsc::Sender<WsMessage>, script_instance: Vec<ScriptedAction>) -> Self {
        Self {
            sender_to_client_ws,
            action_queue: VecDeque::from(script_instance),
            client_id: None,
            last_request_id: None,
        }
    }

    async fn pop_action(&mut self) -> Option<ScriptedAction> {
        self.action_queue.pop_front()
    }

    async fn send_ws_message(&self, ws_message: WsMessage) -> Result<(), String> {
        self.sender_to_client_ws
            .send(ws_message)
            .await
            .map_err(|e| format!("Failed to send WsMessage to client_ws_task: {}", e))
    }

    async fn send_server_message(&self, server_message: ServerMessage) -> Result<(), String> {
        match serde_json::to_string(&server_message) {
            Ok(json) => self.send_ws_message(WsMessage::Text(json)).await,
            Err(e) => Err(format!("Failed to serialize server message: {}", e)),
        }
    }

    fn set_client_id(&mut self, client_id: String) {
        self.client_id = Some(client_id);
    }

    #[allow(dead_code)]
    fn get_client_id(&self) -> Option<String> {
        self.client_id.clone()
    }

    fn set_last_request_id(&mut self, request_id: Uuid) {
        self.last_request_id = Some(request_id);
    }

    fn get_last_request_id(&self) -> Option<Uuid> {
        self.last_request_id
    }
}

#[allow(dead_code)]
pub struct MockAuctioneer {
    addr: SocketAddr,
    config: Arc<Mutex<MockServerConfig>>,
    // Used to signal the main listener loop to shut down.
    shutdown_notifier: Arc<Notify>,
    // Handle to the main listener task, so we can await its completion.
    listener_handle: Mutex<Option<JoinHandle<()>>>, // Option to allow taking it in drop
    // Flag to indicate if shutdown has been initiated, to avoid multiple shutdowns.
    is_shutting_down: Arc<AtomicBool>,
}

impl MockAuctioneer {
    pub async fn new(config: MockServerConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let arc_config = Arc::new(Mutex::new(config));
        let shutdown_notifier = Arc::new(Notify::new());
        let is_shutting_down_flag = Arc::new(AtomicBool::new(false));

        let server_config_clone_for_listener = arc_config.clone();
        let listener_shutdown_signal = shutdown_notifier.clone();
        let listener_is_shutting_down_flag = is_shutting_down_flag.clone();

        let listener_handle = tokio::spawn(async move {
            info!("Scripted Mock Auctioneer server listening on {}", addr);
            loop {
                tokio::select! {
                    biased; // Prioritize shutdown signal
                    _ = listener_shutdown_signal.notified() => {
                        info!("Listener task: Shutdown signal received. Stopping listener.");
                        break;
                    }
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, peer_addr)) => {
                                if listener_is_shutting_down_flag.load(Ordering::SeqCst) {
                                    info!("Listener task: Shutdown in progress, rejecting new connection from {}", peer_addr);
                                    drop(stream);
                                    continue;
                                }
                                info!("New client connection from {}", peer_addr);
                                let per_client_config_guard = server_config_clone_for_listener.lock().await;
                                let per_client_config = per_client_config_guard.clone();
                                drop(per_client_config_guard);

                                let client_handler_shutdown_notifier = listener_shutdown_signal.clone();
                                let client_handler_is_shutting_down_flag = listener_is_shutting_down_flag.clone();


                                tokio::spawn(async move {
                                    if let Err(e) = Self::handle_connection(
                                        stream,
                                        peer_addr,
                                        per_client_config,
                                        client_handler_shutdown_notifier,
                                        client_handler_is_shutting_down_flag,
                                    )
                                    .await
                                    {
                                        error!("Handler for {} exited with error: {}", peer_addr, e);
                                    }
                                });
                            }
                            Err(e) => {
                                if listener_is_shutting_down_flag.load(Ordering::SeqCst) {
                                    info!("Listener task: Shutdown in progress, error during accept is expected: {}", e);
                                    break; // Exit loop if shutting down
                                }
                                error!("Failed to accept new connection: {}", e);
                                if !Self::is_recoverable_listener_error(&e) {
                                    error!("Unrecoverable listener error. Shutting down listener task.");
                                    listener_shutdown_signal.notify_one(); // Signal self to stop in case not already.
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            info!("Listener task for {} has shut down.", addr);
        });

        Ok(Self {
            addr,
            config: arc_config,
            shutdown_notifier,
            listener_handle: Mutex::new(Some(listener_handle)),
            is_shutting_down: is_shutting_down_flag,
        })
    }

    fn is_recoverable_listener_error(e: &std::io::Error) -> bool {
        match e.kind() {
            _ => false,
        }
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}", self.addr)
    }

    #[allow(dead_code)]
    pub async fn update_config(&self, config: MockServerConfig) {
        if self.is_shutting_down.load(Ordering::SeqCst) {
            warn!("Server is shutting down. Cannot update config.");
            return;
        }
        let mut current_config = self.config.lock().await;
        *current_config = config;
    }

    #[allow(dead_code)]
    pub async fn get_config(&self) -> MockServerConfig {
        self.config.lock().await.clone()
    }

    async fn handle_connection(
        stream: TcpStream,
        peer: SocketAddr,
        server_config: MockServerConfig,
        global_shutdown_notifier: Arc<Notify>,
        global_is_shutting_down_flag: Arc<AtomicBool>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ws_stream = match accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                warn!("WebSocket handshake failed for {}: {}", peer, e);
                return Err(Box::new(e));
            }
        };
        info!("WebSocket connection established with {}", peer);
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        let (ctx_sender_to_ws_task, mut ctx_receiver_from_ctx) = mpsc::channel::<WsMessage>(100);

        let mut client_context =
            ClientContext::new(ctx_sender_to_ws_task, server_config.script_template.clone());

        let sender_task_peer = peer; // Clone peer for sender_task logging
        let sender_task = tokio::spawn(async move {
            while let Some(ws_msg) = ctx_receiver_from_ctx.recv().await {
                if let Err(e) = ws_sender.send(ws_msg).await {
                    // This can happen normally if the client disconnects or script closes.
                    debug!(
                        "Client {}: Failed to send message via ws_sender (connection likely closed by script or client): {}",
                        sender_task_peer, e
                    );
                    break;
                }
            }
            debug!(
                "Sender task for {} shutting down. Attempting to close ws_sender.",
                sender_task_peer
            );
            if let Err(e) = ws_sender.close().await {
                debug!(
                    "Client {}: Error closing ws_sender (may be already closed): {}",
                    sender_task_peer, e
                );
            }
        });

        'script_loop: while let Some(action) = client_context.pop_action().await {
            if global_is_shutting_down_flag.load(Ordering::Relaxed)
                && !matches!(action, ScriptedAction::ServerDown)
            {
                info!("Client {}: Server is shutting down, aborting script early.", peer);
                break 'script_loop;
            }

            info!("Client {}: Executing script action: {:?}", peer, action);
            match action {
                ScriptedAction::ExpectClientMessage {
                    matcher,
                    response: scripted_response_result,
                    timeout_duration,
                } => {
                    let overall_timeout_duration =
                        timeout_duration.unwrap_or_else(|| Duration::from_secs(10));
                    let deadline = tokio::time::Instant::now() + overall_timeout_duration;

                    let received_text_payload: Option<String>;

                    'receive_text_loop: loop {
                        if global_is_shutting_down_flag.load(Ordering::Relaxed) {
                            info!("Client {}: Server is shutting down during ExpectClientMessage. Aborting script.", peer);
                            break 'script_loop;
                        }
                        let now = tokio::time::Instant::now();
                        if now >= deadline {
                            warn!(
                                "Client {}: Overall timeout ({:?}) reached while waiting for message matching {:?}. Aborting script.",
                                peer, overall_timeout_duration, matcher
                            );
                            break 'script_loop;
                        }

                        let remaining_time_for_read = deadline - now;

                        match timeout(remaining_time_for_read, ws_receiver.next()).await {
                            Ok(Some(Ok(ws_msg))) => match ws_msg {
                                WsMessage::Text(text) => {
                                    debug!("Client {}: Received Text message.", peer);
                                    received_text_payload = Some(text);
                                    break 'receive_text_loop;
                                }
                                WsMessage::Close(close_frame) => {
                                    info!(
                                        "Client {}: Sent Close frame: {:?}. Aborting script.",
                                        peer, close_frame
                                    );
                                    break 'script_loop;
                                }
                                WsMessage::Ping(ping_data) => {
                                    debug!("Client {}: Received Ping from client. Sending Pong.", peer);
                                    if client_context
                                        .send_ws_message(WsMessage::Pong(ping_data))
                                        .await
                                        .is_err()
                                    {
                                        error!(
                                            "Client {}: Failed to send Pong to client. Aborting script.",
                                            peer
                                        );
                                        break 'script_loop;
                                    }
                                }
                                WsMessage::Pong(_) => {
                                    debug!("Client {}: Received Pong from client.", peer);
                                }
                                WsMessage::Binary(_) => {
                                    debug!("Client {}: Received Binary message, ignoring.", peer);
                                }
                                WsMessage::Frame(_) => {
                                    debug!("Client {}: Received low-level Frame, ignoring.", peer);
                                }
                            },
                            Ok(Some(Err(e))) => {
                                error!("Client {}: WebSocket error while receiving message: {}. Aborting script.", peer, e);
                                break 'script_loop;
                            }
                            Ok(None) => {
                                info!(
                                    "Client {}: WebSocket stream ended (disconnected). Aborting script.",
                                    peer
                                );
                                break 'script_loop;
                            }
                            Err(_elapsed) => {
                                warn!(
                                    "Client {}: Timeout on individual read for {:?}, overall deadline likely hit. Aborting script.",
                                    peer, matcher
                                );
                                break 'script_loop;
                            }
                        }
                    }

                    let received_text_payload = received_text_payload.unwrap();
                    match serde_json::from_str::<ClientMessage>(&received_text_payload) {
                        Ok(client_msg) => {
                            let (actual_msg_type, opt_req_id) = match &client_msg {
                                ClientMessage::Register(_) => (ClientMessageTypeMatcher::Register, None),
                                ClientMessage::Bid(_) => (ClientMessageTypeMatcher::Bid, None),
                                ClientMessage::Solve(_) => (ClientMessageTypeMatcher::Solve, None),
                                ClientMessage::Quote(m) => {
                                    (ClientMessageTypeMatcher::Quote, Some(m.request_id))
                                }
                            };
                            if let Some(req_id) = opt_req_id {
                                client_context.set_last_request_id(req_id);
                            }

                            let matches_expectation = match &matcher {
                                ClientMessageMatcher::Any => true,
                                ClientMessageMatcher::ByType(expected_type) => {
                                    actual_msg_type == *expected_type
                                }
                            };

                            if matches_expectation {
                                debug!(
                                    "Client {}: Message matched {:?}. Content: {:?}",
                                    peer, matcher, client_msg
                                );
                                if let ClientMessage::Register(ref reg_msg) = client_msg {
                                    client_context.set_client_id(reg_msg.solver_id.clone());
                                }

                                match scripted_response_result.clone() {
                                    Ok(Some(server_response)) => {
                                        if let Err(e) =
                                            client_context.send_server_message(server_response).await
                                        {
                                            error!("Client {}: Error sending scripted response: {}. Aborting script.", peer, e);
                                            break 'script_loop;
                                        }
                                    }
                                    Ok(None) => { /* No response */ }
                                    Err(mut server_error) => {
                                        if server_error.request_id.is_none() {
                                            server_error.request_id = client_context.get_last_request_id();
                                        }
                                        if let Err(e) = client_context
                                            .send_server_message(ServerMessage::Error(server_error))
                                            .await
                                        {
                                            error!("Client {}: Error sending scripted error response: {}. Aborting script.", peer, e);
                                            break 'script_loop;
                                        }
                                    }
                                }
                            } else {
                                warn!("Client {}: Received message did not match expectation. Expected {:?}, got {:?}. Content: {:?}. Aborting script.", peer, matcher, actual_msg_type, client_msg);
                                break 'script_loop;
                            }
                        }
                        Err(e) => {
                            error!("Client {}: Failed to deserialize client message from text: '{}'. Error: {}. Aborting script.", peer, received_text_payload, e);
                            break 'script_loop;
                        }
                    }
                }
                ScriptedAction::SendServerMessage(server_msg) => {
                    if let Err(e) = client_context.send_server_message(server_msg).await {
                        error!(
                            "Client {}: Error sending proactive server message: {}. Aborting script.",
                            peer, e
                        );
                        break 'script_loop;
                    }
                }
                ScriptedAction::Delay(duration) => {
                    sleep(duration).await;
                }
                ScriptedAction::CloseConnectionGracefully => {
                    info!("Client {}: Script orders graceful close.", peer);
                    if client_context
                        .send_ws_message(WsMessage::Close(None))
                        .await
                        .is_err()
                    {
                        warn!(
                            "Client {}: Failed to send Close frame, ws_sender task might be down.",
                            peer
                        );
                    }
                    break 'script_loop;
                }
                ScriptedAction::DropConnectionAbruptly => {
                    info!(
                        "Client {}: Script orders abrupt drop. Aborting sender task and handler.",
                        peer
                    );
                    sender_task.abort();
                    return Ok(()); // Exit handle_connection immediately.
                }
                ScriptedAction::BecomeUnresponsive => {
                    info!("Client {}: Script orders to become unresponsive.", peer);
                    loop {
                        if global_is_shutting_down_flag.load(Ordering::Relaxed) {
                            info!("Client {}: Server is shutting down while unresponsive. Exiting unresponsive loop.", peer);
                            sender_task.abort(); // ensure sender is cleaned up
                            return Ok(()); // Exit handler
                        }
                        tokio::select! {
                            biased;
                            maybe_msg = ws_receiver.next() => {
                                match maybe_msg {
                                    Some(Ok(WsMessage::Close(_))) | None => {
                                        info!("Client {} disconnected while server was unresponsive.", peer);
                                        sender_task.abort();
                                        return Ok(());
                                    }
                                    Some(Ok(other_msg)) => {
                                        debug!("Client {}: Received message while unresponsive: {:?}", peer, other_msg);
                                    }
                                    Some(Err(e)) => {
                                        warn!("Client {}: WebSocket error while unresponsive: {}.", peer, e);
                                        sender_task.abort();
                                        return Ok(());
                                    }
                                }
                            }
                            _ = sleep(Duration::from_secs(2)) => {
                                debug!("Client {}: Still unresponsive as per script. Global shutdown: {}", peer, global_is_shutting_down_flag.load(Ordering::Relaxed));
                            }
                        }
                    }
                }
                ScriptedAction::ServerDown => {
                    info!(
                        "Client {}: Script orders SERVER DOWN. Signaling listener to stop.",
                        peer
                    );
                    // Set the global flag first to prevent new connections immediately
                    global_is_shutting_down_flag.store(true, Ordering::SeqCst);
                    global_shutdown_notifier.notify_one(); // Signal the main listener
                    info!(
                        "Client {}: ServerDown action processed. Terminating this client handler.",
                        peer
                    );
                    sender_task.abort();
                    return Ok(());
                }
            }
        }

        info!(
            "Client {}: Script finished or aborted. Cleaning up connection.",
            peer
        );

        drop(client_context); // This closes the mpsc channel to sender_task

        if let Err(e) = sender_task.await {
            if e.is_cancelled() {
                debug!(
                    "Client {}: Sender task was cancelled (expected on abrupt drop or server down).",
                    peer
                );
            } else {
                error!("Client {}: Sender task panicked: {:?}", peer, e);
            }
        } else {
            debug!("Client {}: Sender task completed gracefully.", peer);
        }
        debug!("Connection handler for {} finished.", peer);
        Ok(())
    }
}
