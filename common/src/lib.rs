use borsh::{BorshDeserialize, BorshSerialize};
use ruint::aliases::U256;

#[derive(Debug)]
pub struct Intent {
    pub intent_id: String,
    pub user_in: String,
    pub user_out: String,
    pub token_in: String,
    pub amount_in: U256,
    pub token_out: String,
    pub amount_out: U256,
    pub winner_solver: String,
    pub timeout: u64,
    pub single_domain: bool,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct UserIntent {
    pub token_in: String,
    pub amount_in: String,
    pub token_out: String,
    pub amount_out: String,
    pub user_address: String,
}
