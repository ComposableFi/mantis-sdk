pub mod http;
pub mod ws;

use anyhow::{Context, Error};
use num::{BigUint, Num};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString, FromRepr};
use utoipa::ToSchema;
pub use validator::Validate;

pub const API_VERSION: &str = "v1-beta";

#[derive(
    Debug, Display, FromRepr, Clone, Copy, EnumString, Serialize, Deserialize, PartialEq, Eq, ToSchema,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum IntentChain {
    Ethereum = 1,
    Solana = 2,
    Base = 3,
}

impl From<IntentChain> for u8 {
    fn from(chain: IntentChain) -> Self {
        chain as u8
    }
}

impl TryFrom<u8> for IntentChain {
    type Error = Error;

    fn try_from(id: u8) -> Result<Self, Self::Error> {
        IntentChain::from_repr(id).context("invalid intent chain id")
    }
}

/// Custom serialization module for the BigUint type from/to a decimal string.
pub mod biguint {
    use super::*;

    pub fn serialize<S>(n: &BigUint, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&n.to_str_radix(10))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BigUint, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        BigUint::from_str_radix(&s, 10).map_err(serde::de::Error::custom)
    }
}
