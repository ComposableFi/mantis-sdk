pub mod ws;

// Re-export the client struct for easier access
pub use ws::AuctioneerWsClient;

// Re-export the auctioneer_api types for easier use
pub use auctioneer_api::IntentChain;