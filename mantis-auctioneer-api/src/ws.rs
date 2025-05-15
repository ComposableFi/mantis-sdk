use std::str::FromStr;

use alloy::hex;
use alloy::primitives::{keccak256, Address, FixedBytes};
use alloy::signers::{Signature, SignerSync};
use anyhow::{anyhow, Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use uuid::Uuid;
use validator::{Validate, ValidationError};

pub use crate::IntentChain;

pub trait SignableMessage {
    fn signature(&self) -> &Option<String>;

    fn signature_mut(&mut self) -> &mut Option<String>;

    fn hash(&self) -> Result<FixedBytes<32>>;

    fn signed<S: SignerSync>(mut self, signer: S) -> Result<Self>
    where
        Self: Sized,
    {
        let hash = self.hash()?;
        let signature = self.signature_mut();
        *signature = Some(hex::encode(signer.sign_hash_sync(&hash)?.as_bytes()));
        Ok(self)
    }

    fn verify(&self, expected_address: Address) -> Result<()> {
        if let Some(signature) = self.signature() {
            let hash = self.hash()?;
            let signature = Signature::from_str(signature)?;
            let recovered_address = signature.recover_address_from_prehash(&hash)?;
            if expected_address != recovered_address {
                return Err(anyhow!("Recovered address does not match the expected address"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    AuctionStart(ServerAuctionStartMessage),
    AuctionResult(ServerAuctionResultMessage),
    Quote(ServerQuoteMessage),
    UnlockedFunds(ServerUnlockedFundsMessage),
    Error(ServerErrorMessage),
}

impl ServerMessage {
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(Error::from)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Register(ClientRegisterMessage),
    Bid(ClientBidMessage),
    Solve(ClientSolveMessage),
    Quote(ClientQuoteMessage),
}

impl ClientMessage {
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(Error::from)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerAuctionStartMessage {
    pub intent_id: u64,
    pub intent: SwapIntent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapIntent {
    pub src_chain: IntentChain,
    pub dst_chain: IntentChain,
    pub src_user: String,
    pub dst_user: String,
    pub token_in: String,
    pub amount_in: String,
    pub token_out: String,
    pub amount_out: String,
    pub timeout: u64,
}

impl SwapIntent {
    pub fn is_single_domain(&self) -> bool {
        self.src_chain == self.dst_chain
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerAuctionResultMessage {
    pub won: bool,
    pub intent_id: u64,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerQuoteMessage {
    pub request_id: Uuid,
    pub intent: SwapIntent,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerUnlockedFundsMessage {
    pub src_chain_id: u8,
    pub solver: String,
    pub intent_id: u64,
    pub token_in: String,
    pub amount_in: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerErrorMessage {
    pub code: u16,
    pub message: String,
    pub request_id: Option<Uuid>,
}

impl ServerErrorMessage {
    pub fn new(code: u16, message: String, request_id: Option<Uuid>) -> Self {
        ServerErrorMessage {
            code,
            message,
            request_id,
        }
    }

    pub fn invalid_message(request_id: Option<Uuid>) -> Self {
        ServerErrorMessage {
            code: 400,
            message: "Invalid message".to_string(),
            request_id,
        }
    }

    pub fn validation_failure(request_id: Option<Uuid>) -> Self {
        ServerErrorMessage {
            code: 400,
            message: "Message validation failed".to_string(),
            request_id,
        }
    }

    pub fn invalid_signature(request_id: Option<Uuid>) -> Self {
        ServerErrorMessage {
            code: 400,
            message: "Invalid message signature".to_string(),
            request_id,
        }
    }

    pub fn missing_field(field: String, request_id: Option<Uuid>) -> Self {
        ServerErrorMessage {
            code: 400,
            message: format!("Missing {} message field", field),
            request_id,
        }
    }

    pub fn unregistered_solver(solver_id: String, request_id: Option<Uuid>) -> Self {
        ServerErrorMessage {
            code: 403,
            message: format!("Solver {} is not registered", solver_id),
            request_id,
        }
    }

    pub fn bannded_solver(solver_id: String, request_id: Option<Uuid>) -> Self {
        ServerErrorMessage {
            code: 403,
            message: format!("Solver {} is banned", solver_id),
            request_id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Validate)]
pub struct ClientRegisterMessage {
    #[validate(length(min = 5, max = 15), custom(function = "validate_alphanumeric"))]
    pub solver_id: String,
    pub solver_addresses: SolverAddresses,
    pub signature: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SolverAddresses {
    pub ethereum: Address,
    pub solana: Pubkey,
    pub base: Address,
}

fn validate_alphanumeric(solver_id: &str) -> Result<(), ValidationError> {
    let alphanumeric = Regex::new(r"^[a-zA-Z0-9]+$").expect("could not compile regex");
    if !alphanumeric.is_match(solver_id) {
        return Err(ValidationError::new("not alphanumeric"));
    }
    Ok(())
}

impl ClientRegisterMessage {
    pub fn new(solver_id: String, solver_addresses: SolverAddresses) -> Self {
        ClientRegisterMessage {
            solver_id,
            solver_addresses,
            signature: None,
        }
    }
}

impl SignableMessage for ClientRegisterMessage {
    fn hash(&self) -> Result<FixedBytes<32>> {
        let message = Self {
            signature: None,
            ..self.clone()
        };

        Ok(keccak256(serde_json::to_string(&message)?))
    }

    fn signature(&self) -> &Option<String> {
        &self.signature
    }

    fn signature_mut(&mut self) -> &mut Option<String> {
        &mut self.signature
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientBidMessage {
    pub solver_id: String,
    pub intent_id: u64,
    pub amount: String,
    pub signature: Option<String>,
}

impl ClientBidMessage {
    pub fn new(solver_id: String, intent_id: u64, amount: String) -> Self {
        ClientBidMessage {
            solver_id,
            intent_id,
            amount,
            signature: None,
        }
    }
}

impl SignableMessage for ClientBidMessage {
    fn hash(&self) -> Result<FixedBytes<32>> {
        let message = Self {
            signature: None,
            ..self.clone()
        };

        Ok(keccak256(serde_json::to_string(&message)?))
    }

    fn signature(&self) -> &Option<String> {
        &self.signature
    }

    fn signature_mut(&mut self) -> &mut Option<String> {
        &mut self.signature
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientSolveMessage {
    pub solver_id: String,
    pub intent_id: u64,
    pub solve_transaction: String,
    pub signature: Option<String>,
}

impl ClientSolveMessage {
    pub fn new(solver_id: String, intent_id: u64, solve_transaction: String) -> Self {
        ClientSolveMessage {
            solver_id,
            intent_id,
            solve_transaction,
            signature: None,
        }
    }
}

impl SignableMessage for ClientSolveMessage {
    fn hash(&self) -> Result<FixedBytes<32>> {
        let message = Self {
            signature: None,
            ..self.clone()
        };

        Ok(keccak256(serde_json::to_string(&message)?))
    }

    fn signature(&self) -> &Option<String> {
        &self.signature
    }

    fn signature_mut(&mut self) -> &mut Option<String> {
        &mut self.signature
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientQuoteMessage {
    pub request_id: Uuid,
    pub src_chain: String,
    pub dst_chain: String,
    pub solver_id: String,
    pub token_in: String,
    pub amount_in: String,
    pub token_out: String,
    pub amount_out: String,
}

#[cfg(test)]
mod tests {
    use alloy::signers::local::PrivateKeySigner;

    use super::*;

    #[test]
    fn test_verify_signature() -> Result<()> {
        let private_key = "2533129b71c9e08d2a1174ac943dfaf699d5d148debe38ef5192db7e84efcf1c";
        let signer = PrivateKeySigner::from_str(private_key)?;
        let expected_address = signer.address();
        let message = ClientRegisterMessage::new(
            "123456".into(),
            SolverAddresses {
                ethereum: signer.address(),
                solana: Pubkey::from_str_const("5zCZ3jk8EZnJyG7fhDqD6tmqiYTLZjik5HUpGMnHrZfC"),
                base: signer.address(),
            },
        );

        let signed = message.signed(signer)?;

        signed.verify(expected_address)?;

        Ok(())
    }
}
