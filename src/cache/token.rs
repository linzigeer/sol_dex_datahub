use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use solana_sdk::pubkey::Pubkey;

use super::RedisCacheRecord;

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenRecord {
    #[serde_as(as = "DisplayFromStr")]
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub decimals: u8,
    pub total_supply: u64,
}

impl RedisCacheRecord for TokenRecord {
    fn key(&self) -> String {
        format!("{}{}", Self::prefix(), self.mint)
    }

    fn prefix() -> &'static str {
        "token:"
    }
}
