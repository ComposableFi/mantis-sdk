pub mod http;
pub mod ws;

// Re-export the client structs for easier access
pub use http::AuctioneerHttpClient;
pub use ws::AuctioneerWsClient;

// Re-export the auctioneer_api types for easier use
pub use auctioneer_api::IntentChain;
