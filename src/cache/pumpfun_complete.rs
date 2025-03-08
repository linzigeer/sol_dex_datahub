use chrono::{DateTime, Utc, serde::ts_seconds};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use solana_sdk::pubkey::Pubkey;

use crate::{common::TxBaseMetaInfo, pumpfun::event::CompleteEvent};

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct PumpfunCompleteRecord {
    #[serde(with = "ts_seconds")]
    pub blk_ts: DateTime<Utc>,
    pub slot: u64,
    pub txid: String,
    pub idx: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub user: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub mint: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub bonding_curve: Pubkey,
}

impl PumpfunCompleteRecord {
    pub fn new(meta: TxBaseMetaInfo, complete_evt: &CompleteEvent) -> Self {
        let TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        } = meta;

        Self {
            blk_ts,
            slot,
            txid,
            idx,
            user: complete_evt.user,
            mint: complete_evt.mint,
            bonding_curve: complete_evt.bonding_curve,
        }
    }
}
