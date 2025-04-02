use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use strum::{Display, EnumString};

pub const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display, EnumString)]
pub enum Dex {
    RaydiumAmm,
    Pumpfun,
    PumpAmm,
    MeteoraDlmm,
    MeteoraDamm,
}

#[derive(Debug, Clone)]
pub struct TxBaseMetaInfo {
    pub blk_ts: DateTime<Utc>,
    pub slot: u64,
    pub txid: String,
    pub idx: u64,
}

pub mod utils {
    pub fn calc_price_sol(sol_amount: u64, token_amount: u64, token_decimals: u8) -> f64 {
        let sol_amount = sol_amount as f64 / 1_000_000_000.0f64;

        let token_amount = token_amount as f64 / 10u64.pow(token_decimals as u32) as f64;

        sol_amount / token_amount
    }
}
