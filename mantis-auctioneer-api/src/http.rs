use std::str::FromStr;

use alloy::hex::FromHexError;
use alloy::primitives::Address;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{BoxError, Json};
use num::bigint::ParseBigIntError;
use num::BigUint;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::{ParsePubkeyError, Pubkey};
use strum::EnumString;
use tracing::error;
use utoipa::ToSchema;
use validator::{Validate, ValidationError, ValidationErrors};

use crate::{biguint, IntentChain};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CheckHealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct ListQuotesQuery {
    pub src_chain: IntentChain,
    pub dst_chain: IntentChain,
    #[validate(custom(function = "validate_token_in"))]
    pub token_in: String,
    #[schema(value_type = String)]
    #[serde(with = "biguint")]
    #[validate(custom(function = "validate_token_in_amount"))]
    pub token_in_amount: BigUint,
    #[validate(custom(function = "validate_token_out"))]
    pub token_out: String,
}

fn validate_token_in_amount(token_in_amount: &BigUint) -> Result<(), ValidationError> {
    if *token_in_amount == BigUint::ZERO {
        return Err(ValidationError::new("token_in_amount cannot be 0"));
    }
    Ok(())
}

fn validate_token_in(token_in: &str) -> Result<(), ValidationError> {
    let address_result = Address::from_str(token_in);
    let pubkey_result = Pubkey::from_str(token_in);

    if address_result.is_err() && pubkey_result.is_err() {
        return Err(ValidationError::new("token_in is not a valid token address"));
    }
    Ok(())
}

fn validate_token_out(token_out: &str) -> Result<(), ValidationError> {
    let address_result = Address::from_str(token_out);
    let pubkey_result = Pubkey::from_str(token_out);

    if address_result.is_err() && pubkey_result.is_err() {
        return Err(ValidationError::new("token_out is not a valid token address"));
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListQuotesResponse {
    pub src_chain: String,
    pub dst_chain: String,
    pub solver_quotes: Vec<SolverQuote>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct ListFeesQuery {
    #[validate(custom(function = "validate_authority"))]
    pub authority: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListFeesResponse {
    pub solana: Vec<SolanaTokenFee>,
    pub ethereum: Vec<EthereumTokenFee>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
pub struct SolanaTokenFee {
    pub token: String,
    pub symbol: Option<String>,
    pub balance: String,
    pub value: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
pub struct EthereumTokenFee {
    pub token: String,
    pub symbol: Option<String>,
    pub balance: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListSwapIntentsQuery {
    pub src_chain: Option<IntentChain>,
    pub period: Option<Period>,
    #[serde(default)]
    pub src_user: Vec<String>,
    pub page: Option<u16>,
    pub page_size: Option<u16>,
}

#[derive(Debug, Clone, Copy, EnumString, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "snake_case")]
pub enum Period {
    All,
    OneDay,
    OneWeek,
    OneMonth,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListSwapIntentsResponse {
    pub page: u16,
    pub items: u16,
    pub page_size: u16,
    pub page_max: u64,
    pub items_max: u64,
    pub intents: Vec<SwapIntent>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetSwapIntentResponse {
    pub intent: SwapIntent,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
pub struct SwapIntent {
    pub intent_id: u64,
    pub created_at: String,
    pub escrow_transaction: String,
    pub src_user: String,
    pub dst_user: String,
    pub src_chain: String,
    pub dst_chain: String,
    pub token_in: String,
    pub amount_in: String,
    pub token_out: String,
    pub amount_wanted: String,
    pub amount_provided: Option<String>,
    pub fee_amount: String,
    pub timeout_sec: u64,
    pub is_canceled: bool,
    pub is_solved: bool,
    pub ai_agent: bool,
    pub solver: Option<String>,
    pub canceled_at: Option<String>,
    pub solved_at: Option<String>,
    pub solve_transaction: Option<String>,
    pub token_in_price_usd: Option<f64>,
    pub token_out_price_usd: Option<f64>,
    pub token_in_symbol: Option<String>,
    pub token_out_symbol: Option<String>,
    pub token_in_decimals: Option<i16>,
    pub token_out_decimals: Option<i16>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct RescanQuery {
    #[validate(custom(function = "validate_authority"))]
    pub authority: String,
    pub src_chain: IntentChain,
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RescanResponse {}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UnlockQuery {
    #[validate(custom(function = "validate_authority"))]
    pub authority: String,
    #[validate(custom(function = "validate_intent_id"))]
    pub intent_id: u64,
    pub src_chain: IntentChain,
    pub token_out: String,
    pub amount_out: String,
    pub dst_user: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UnlockResponse {
    pub transaction: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct CancelQuery {
    #[validate(custom(function = "validate_authority"))]
    pub authority: String,
    #[validate(custom(function = "validate_intent_id"))]
    pub intent_id: u64,
    pub src_chain: IntentChain,
    pub token_in_mint: Option<String>,
    pub src_user: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CancelResponse {
    pub transaction: String,
}

fn validate_authority(authority: &str) -> Result<(), ValidationError> {
    if authority != "4e6d9d0849740b385d60c59fded9ee97" {
        return Err(ValidationError::new("Invalid authority"));
    }
    Ok(())
}

pub fn validate_intent_id(intent_id: u64) -> Result<(), ValidationError> {
    if !(100_000_000_000..=999_999_999_999).contains(&intent_id) {
        return Err(ValidationError::new("intent_id is not a 12-digit number"));
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetStatsQuery {
    pub period: Option<Period>,
    pub src_chain: Option<IntentChain>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetStatsResponse {
    pub total_trades: u64,
    pub unique_addresses: u64,
    pub total_volume: f64,
    pub total_local_volume: f64,
    pub total_remote_volume: f64,
    pub total_value_in: f64,
    pub total_value_out: f64,
    pub total_fees: f64,
    pub top_assets: Vec<StatsAsset>,
    pub top_solvers: Vec<StatsSolver>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsSolver {
    pub address: String,
    pub volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsAsset {
    pub address: String,
    pub symbol: Option<String>,
    pub volume: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetTimeSeriesQuery {
    pub period: Option<Period>,
    pub src_chain: Option<IntentChain>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetTimeSeriesResponse {
    pub start_timestamp: u64,
    pub end_timestamp: u64,
    pub total_trades: Vec<i64>,
    pub total_volume: Vec<f64>,
    pub total_value_in: Vec<f64>,
    pub total_value_out: Vec<f64>,
    pub total_fees: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SolverQuote {
    pub solver_id: String,
    pub token_in: String,
    pub token_in_amount: String,
    pub token_out: String,
    pub token_out_amount: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ServerErrorResponse {
    pub code: u16,
    pub message: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationErrors),
    #[error("Parse big int error: {0}")]
    ParseBigInt(#[from] ParseBigIntError),
    #[error("Parse pubkey error: {0}")]
    ParsePubkey(#[from] ParsePubkeyError),
    #[error("Parse error: {0}")]
    ParseStrum(#[from] strum::ParseError),
    #[error("Parse address error: {0}")]
    ParseAddressError(#[from] FromHexError),
    #[error("Not found: {0}")]
    NotFound(BoxError),
    #[error("Request timeout: {0}")]
    Timeout(BoxError),
    #[error("Rate limit reached: {0}")]
    Ratelimit(BoxError),
    #[error("Internal error: {0}")]
    InternalBoxed(#[from] BoxError),
    #[error("Internal error: {0}")]
    InternalAnyhow(#[from] anyhow::Error),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        error!("Server error: {:#}", &self);
        let (status, error_message) = match self {
            Self::Validation(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            Self::ParseBigInt(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            Self::ParsePubkey(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            Self::ParseStrum(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            Self::ParseAddressError(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            Self::NotFound(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            Self::Timeout(error) => (StatusCode::REQUEST_TIMEOUT, error.to_string()),
            Self::Ratelimit(error) => (StatusCode::TOO_MANY_REQUESTS, error.to_string()),
            Self::InternalBoxed(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
            Self::InternalAnyhow(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        };

        let body = Json(ServerErrorResponse {
            code: status.as_u16(),
            message: error_message,
        });

        (status, body).into_response()
    }
}
