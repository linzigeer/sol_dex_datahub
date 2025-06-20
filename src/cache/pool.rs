use std::str::FromStr;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc, serde::ts_seconds};
use redis::aio::MultiplexedConnection;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use solana_sdk::pubkey::Pubkey;

use crate::{
    common::{Dex, TxBaseMetaInfo, WSOL_MINT},
    meteora::{
        damm::{
            event::MeteoraDammPoolCreated,
            instruction::{INIT_WITH_CONFIG_IX_ID, INIT_WITH_CONFIG2_IX_ID},
        },
        dlmm::event::MeteoraLbPairCreateEvent,
    },
    pumpamm::event::PumpAmmCreatePoolEvent,
    pumpfun::event::CreateEvent,
    qn_req_processor::IxAccount,
    raydium::event::InitLog,
};

use super::RedisCacheRecord;

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct DexPoolCreatedRecord {
    #[serde(with = "ts_seconds")]
    pub blk_ts: DateTime<Utc>,
    pub slot: u64,
    pub txid: String,
    pub idx: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub creator: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub addr: Pubkey,
    pub dex: Dex,
    #[serde_as(as = "DisplayFromStr")]
    pub mint_a: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub mint_b: Pubkey,
    pub decimals_a: u8,
    pub decimals_b: u8,
}

pub const DEX_POOL_RECORD_EXP_SECS: u64 = 3600 * 12;

impl DexPoolCreatedRecord {
    pub fn is_wsol_pool(&self) -> bool {
        self.mint_a == WSOL_MINT || self.mint_b == WSOL_MINT
    }

    pub fn as_pool_record(&self) -> DexPoolRecord {
        DexPoolRecord {
            addr: self.addr,
            dex: self.dex,
            is_complete: false,
            mint_a: self.mint_a,
            mint_b: self.mint_b,
            decimals_a: self.decimals_a,
            decimals_b: self.decimals_b,
        }
    }

    pub fn from_pumpfun_create_log(tx_meta: TxBaseMetaInfo, log: CreateEvent) -> Self {
        let TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        } = tx_meta;

        DexPoolCreatedRecord {
            blk_ts,
            slot,
            txid,
            idx,
            addr: log.bonding_curve,
            creator: log.user,
            dex: Dex::Pumpfun,
            mint_a: log.mint,
            mint_b: WSOL_MINT,
            decimals_a: 6,
            decimals_b: 9,
        }
    }

    pub fn from_pumpamm_create_log(tx_meta: TxBaseMetaInfo, log: PumpAmmCreatePoolEvent) -> Self {
        let TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        } = tx_meta;

        DexPoolCreatedRecord {
            blk_ts,
            slot,
            txid,
            idx,
            addr: log.pool,
            creator: log.creator,
            dex: Dex::PumpAmm,
            mint_a: log.base_mint,
            mint_b: log.quote_mint,
            decimals_a: log.base_mint_decimals,
            decimals_b: log.quote_mint_decimals,
        }
    }

    pub fn from_raydium_init_log(
        tx_meta: TxBaseMetaInfo,
        log: InitLog,
        accounts: &[IxAccount],
    ) -> Result<Self> {
        let amm_acc = accounts
            .get(4)
            .ok_or_else(|| anyhow!("need amm addr in init raydium instruction accounts"))?;
        let amm_pubkey = Pubkey::from_str(&amm_acc.pubkey)?;
        let coin_mint_acc = accounts
            .get(8)
            .ok_or_else(|| anyhow!("need coin mint in init raydium instruction accounts"))?;
        let coin_mint_pubkey = Pubkey::from_str(&coin_mint_acc.pubkey)?;
        let pc_mint_acc = accounts
            .get(9)
            .ok_or_else(|| anyhow!("need pc mint in init raydium instruction accounts"))?;
        let pc_mint_pubkey = Pubkey::from_str(&pc_mint_acc.pubkey)?;
        let creator_acc = accounts
            .get(17)
            .ok_or_else(|| anyhow!("need pool creator in init raydium instruction accounts"))?;
        let creator_pubkey = Pubkey::from_str(&creator_acc.pubkey)?;

        let TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        } = tx_meta;
        Ok(Self {
            blk_ts,
            slot,
            txid,
            idx,
            addr: amm_pubkey,
            creator: creator_pubkey,
            dex: Dex::RaydiumAmm,
            mint_a: coin_mint_pubkey,
            mint_b: pc_mint_pubkey,
            decimals_a: log.coin_decimals,
            decimals_b: log.pc_decimals,
        })
    }

    pub fn from_meteora_dlmm_lp_create_log(
        tx_meta: TxBaseMetaInfo,
        log: MeteoraLbPairCreateEvent,
        accounts: &[IxAccount],
    ) -> Result<Self> {
        let TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        } = tx_meta;

        let MeteoraLbPairCreateEvent {
            lb_pair,
            token_x,
            token_y,
            ..
        } = log;

        let x_vault_acc = accounts.get(4).ok_or_else(|| {
            anyhow!("need x vault in meteora dlmm create lb pair instruction accounts")
        })?;
        let x_vault_token_amt = x_vault_acc
            .post_amt
            .token
            .clone()
            .ok_or_else(|| anyhow!("meteora dlmm x vault should have token amt"))?;

        let y_vault_acc = accounts.get(5).ok_or_else(|| {
            anyhow!("need y vault in meteora dlmm create lb pair instruction accounts")
        })?;
        let y_vault_token_amt = y_vault_acc
            .post_amt
            .token
            .clone()
            .ok_or_else(|| anyhow!("meteora dlmm y vault should have token amt"))?;

        let creator_acc = accounts.get(8).ok_or_else(|| {
            anyhow!("need pool creator in meteora dlmm create lb pair instruction accounts")
        })?;
        let creator_pubkey = Pubkey::from_str(&creator_acc.pubkey)?;

        Ok(Self {
            blk_ts,
            slot,
            txid,
            idx,
            addr: lb_pair,
            creator: creator_pubkey,
            dex: Dex::MeteoraDlmm,
            mint_a: token_x,
            mint_b: token_y,
            decimals_a: x_vault_token_amt.decimals,
            decimals_b: y_vault_token_amt.decimals,
        })
    }

    pub fn from_meteora_damm_pool_create_log(
        tx_meta: TxBaseMetaInfo,
        log: MeteoraDammPoolCreated,
        accounts: &[IxAccount],
        ix_data: &str,
    ) -> Result<Self> {
        let TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        } = tx_meta;

        let MeteoraDammPoolCreated {
            pool,
            token_a_mint,
            token_b_mint,
            ..
        } = log;
        let ix_bytes = bs58::decode(ix_data).into_vec()?;
        let has_config = ix_bytes.starts_with(&INIT_WITH_CONFIG_IX_ID)
            || ix_bytes.starts_with(&INIT_WITH_CONFIG2_IX_ID);
        let (token_vault_a_idx, token_vault_b_idx) = if has_config { (7, 8) } else { (6, 7) };

        let a_vault_acc = accounts.get(token_vault_a_idx).ok_or_else(|| {
            anyhow!("need a token vault in meteora damm create pool instruction accounts")
        })?;
        let a_vault_token_amt = a_vault_acc
            .post_amt
            .token
            .clone()
            .ok_or_else(|| anyhow!("meteora damm a valult should have token amt"))?;

        let b_vault_acc = accounts.get(token_vault_b_idx).ok_or_else(|| {
            anyhow!("need b token vault in meteora damm create pool instruction accounts")
        })?;
        let b_vault_token_amt = b_vault_acc
            .post_amt
            .token
            .clone()
            .ok_or_else(|| anyhow!("meteora damm b token valult should have token amt"))?;

        let creator_idx = if has_config { 18 } else { 17 };
        let creator_acc = accounts.get(creator_idx).ok_or_else(|| {
            anyhow!("need pool creator in meteora damm create pool instruction accounts")
        })?;
        let creator_pubkey = Pubkey::from_str(&creator_acc.pubkey)?;

        Ok(Self {
            blk_ts,
            slot,
            txid,
            idx,
            addr: pool,
            creator: creator_pubkey,
            dex: Dex::MeteoraDamm,
            mint_a: token_a_mint,
            mint_b: token_b_mint,
            decimals_a: a_vault_token_amt.decimals,
            decimals_b: b_vault_token_amt.decimals,
        })
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct DexPoolRecord {
    #[serde_as(as = "DisplayFromStr")]
    pub addr: Pubkey,
    pub dex: Dex,
    pub is_complete: bool,
    #[serde_as(as = "DisplayFromStr")]
    pub mint_a: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub mint_b: Pubkey,
    pub decimals_a: u8,
    pub decimals_b: u8,
}

impl DexPoolRecord {
    pub async fn from_meteora_swap_accounts(
        lbpair_pubkey: Pubkey,
        accounts: &[IxAccount],
        redis_conn: &mut MultiplexedConnection,
    ) -> Result<Self> {
        let key = format!("{}{}", DexPoolRecord::prefix(), lbpair_pubkey);
        let mut cached_pool = DexPoolRecord::from_redis(redis_conn, &key).await?;
        if cached_pool.is_none() {
            let token_x_vault = accounts
                .get(2)
                .ok_or_else(|| anyhow!("need token x value in meteora dlmm swap log"))?;
            let pool_token_x_amt = token_x_vault.post_amt.token.clone().ok_or_else(|| {
                anyhow!(
                    "meteora dlmm token x vault {} should have balance",
                    token_x_vault.pubkey
                )
            })?;
            let token_x_mint = Pubkey::from_str(&pool_token_x_amt.mint)?;
            let token_x_decimals = pool_token_x_amt.decimals;

            let token_y_vault = accounts
                .get(3)
                .ok_or_else(|| anyhow!("need token y value in meteora dlmm swap log"))?;
            let pool_token_y_amt = token_y_vault.post_amt.token.clone().ok_or_else(|| {
                anyhow!(
                    "meteora dlmm token y vault {} should have balance",
                    token_y_vault.pubkey
                )
            })?;
            let token_y_mint = Pubkey::from_str(&pool_token_y_amt.mint)?;
            let token_y_decimals = pool_token_y_amt.decimals;
            let pool_record = Self {
                addr: lbpair_pubkey,
                dex: Dex::MeteoraDlmm,
                is_complete: false,
                mint_a: token_x_mint,
                mint_b: token_y_mint,
                decimals_a: token_x_decimals,
                decimals_b: token_y_decimals,
            };
            pool_record
                .save_ex(redis_conn, DEX_POOL_RECORD_EXP_SECS)
                .await?;
            cached_pool = Some(pool_record);
        }
        Ok(cached_pool.unwrap())
    }

    pub async fn from_meteora_damm_swap_accounts(
        pool: Pubkey,
        accounts: &[IxAccount],
        redis_conn: &mut MultiplexedConnection,
    ) -> Result<Self> {
        let key = format!("{}{}", DexPoolRecord::prefix(), pool);
        let mut cached_pool = DexPoolRecord::from_redis(redis_conn, &key).await?;
        if cached_pool.is_none() {
            let token_vault_a = accounts
                .get(5)
                .ok_or_else(|| anyhow!("need token a value in meteora damm swap log"))?;
            let pool_token_a_amt = token_vault_a.post_amt.token.clone().ok_or_else(|| {
                anyhow!(
                    "meteora damm token a vault {} should have balance",
                    token_vault_a.pubkey
                )
            })?;
            let token_a_mint = Pubkey::from_str(&pool_token_a_amt.mint)?;
            let token_a_decimals = pool_token_a_amt.decimals;

            let token_vault_b = accounts
                .get(6)
                .ok_or_else(|| anyhow!("need token b value in meteora damm swap log"))?;
            let pool_token_b_amt = token_vault_b.post_amt.token.clone().ok_or_else(|| {
                anyhow!(
                    "meteora damm token b vault {} should have balance",
                    token_vault_b.pubkey
                )
            })?;
            let token_b_mint = Pubkey::from_str(&pool_token_b_amt.mint)?;
            let token_b_decimals = pool_token_b_amt.decimals;
            let pool_record = Self {
                addr: pool,
                dex: Dex::MeteoraDamm,
                is_complete: false,
                mint_a: token_a_mint,
                mint_b: token_b_mint,
                decimals_a: token_a_decimals,
                decimals_b: token_b_decimals,
            };
            pool_record
                .save_ex(redis_conn, DEX_POOL_RECORD_EXP_SECS)
                .await?;
            cached_pool = Some(pool_record);
        }
        Ok(cached_pool.unwrap())
    }

    pub async fn from_pumpamm_swap_accounts(
        pool_pubkey: Pubkey,
        accounts: &[IxAccount],
        redis_conn: &mut MultiplexedConnection,
    ) -> Result<Self> {
        let key = format!("{}{}", Self::prefix(), pool_pubkey);
        let mut cached_pool = Self::from_redis(redis_conn, &key).await?;
        if cached_pool.is_none() {
            let base_token_vault_idx = 7;
            let quote_token_vault_idx = 8;

            let base_token_vault = accounts
                .get(base_token_vault_idx)
                .ok_or_else(|| anyhow!("need base token vault in pumpamm swap log"))?;
            let base_token_amt = base_token_vault
                .post_amt
                .token
                .clone()
                .ok_or_else(|| anyhow!("base token should have balance in pumpamm swap log"))?;
            let mint_a = Pubkey::from_str(&base_token_amt.mint)?;
            let decimals_a = base_token_amt.decimals;

            let quote_token_vault = accounts
                .get(quote_token_vault_idx)
                .ok_or_else(|| anyhow!("need quote token vault in pumpamm swap log"))?;
            let quote_token_amt =
                quote_token_vault.post_amt.token.clone().ok_or_else(|| {
                    anyhow!("quote token should have balance in pumpamm swap log")
                })?;
            let mint_b = Pubkey::from_str(&quote_token_amt.mint)?;
            let decimals_b = quote_token_amt.decimals;

            let pool_record = Self {
                addr: pool_pubkey,
                dex: Dex::PumpAmm,
                is_complete: false,
                mint_a,
                mint_b,
                decimals_a,
                decimals_b,
            };
            pool_record
                .save_ex(redis_conn, DEX_POOL_RECORD_EXP_SECS)
                .await?;
            cached_pool = Some(pool_record);
        }

        Ok(cached_pool.unwrap())
    }

    pub async fn from_raydium_amm_trade_accounts(
        amm_pubkey: Pubkey,
        accounts: &[IxAccount],
        redis_conn: &mut MultiplexedConnection,
    ) -> Result<Self> {
        let key = format!("{}{}", DexPoolRecord::prefix(), amm_pubkey);
        let mut cached_pool = DexPoolRecord::from_redis(redis_conn, &key).await?;
        if cached_pool.is_none() {
            let mut coin_token_vault_idx = 4;
            let mut pc_token_vault_idx = 5;
            if accounts.len() == 18 {
                coin_token_vault_idx = 5;
                pc_token_vault_idx = 6;
            }

            let coin_token_vault = accounts
                .get(coin_token_vault_idx)
                .ok_or_else(|| anyhow!("need coin token vault in raydium amm swap base in log"))?;
            let coin_token_amt = coin_token_vault.post_amt.token.clone().ok_or_else(|| {
                anyhow!("coin token should have balance in raydium amm base in swap")
            })?;
            let mint_a = Pubkey::from_str(&coin_token_amt.mint)?;
            let decimals_a = coin_token_amt.decimals;
            let pc_token_vault = accounts
                .get(pc_token_vault_idx)
                .ok_or_else(|| anyhow!("need pc token vault in raydium amm swap base in log"))?;
            let pc_token_amt = pc_token_vault.post_amt.token.clone().ok_or_else(|| {
                anyhow!("pc token should have balance in raydium amm base in swap log")
            })?;
            let mint_b = Pubkey::from_str(&pc_token_amt.mint)?;
            let decimals_b = pc_token_amt.decimals;

            let pool_record = Self {
                addr: amm_pubkey,
                dex: Dex::RaydiumAmm,
                is_complete: false,
                mint_a,
                mint_b,
                decimals_a,
                decimals_b,
            };
            pool_record
                .save_ex(redis_conn, DEX_POOL_RECORD_EXP_SECS)
                .await?;
            cached_pool = Some(pool_record);
        }
        Ok(cached_pool.unwrap())
    }

    pub fn from_pumpfun_curve_and_mint(curve: Pubkey, mint: Pubkey, is_complete: bool) -> Self {
        DexPoolRecord {
            addr: curve,
            dex: Dex::Pumpfun,
            is_complete,
            mint_a: mint,
            mint_b: WSOL_MINT,
            decimals_a: 6,
            decimals_b: 9,
        }
    }

    pub async fn from_pumpfun_trade_accounts(
        accounts: &[IxAccount],
        redis_conn: &mut MultiplexedConnection,
    ) -> Result<Self> {
        let curve_acc = accounts
            .get(3)
            .ok_or_else(|| anyhow!("need curve addr in pumpfun trade accounts"))?;
        let curve_pubkey = Pubkey::from_str(&curve_acc.pubkey)?;
        let mint_acc = accounts
            .get(2)
            .ok_or_else(|| anyhow!("need token addr in pumpfun trade accounts"))?;
        let mint_pubkey = Pubkey::from_str(&mint_acc.pubkey)?;
        let key = format!("{}{}", DexPoolRecord::prefix(), curve_pubkey);
        let mut cached_pool = DexPoolRecord::from_redis(redis_conn, &key).await?;
        if cached_pool.is_none() {
            let pool_record = Self {
                addr: curve_pubkey,
                dex: Dex::Pumpfun,
                is_complete: false,
                mint_a: mint_pubkey,
                mint_b: WSOL_MINT,
                decimals_a: 6,
                decimals_b: 9,
            };
            pool_record
                .save_ex(redis_conn, DEX_POOL_RECORD_EXP_SECS)
                .await?;
            cached_pool = Some(pool_record);
        }
        Ok(cached_pool.unwrap())
    }

    pub fn is_wsol_pool(&self) -> bool {
        self.mint_a == WSOL_MINT || self.mint_b == WSOL_MINT
    }

    pub fn is_raydium_buy(&self, direction: u64) -> bool {
        // pc2coin
        if direction == 1 {
            if self.mint_b == WSOL_MINT {
                return true;
            }
            return false;
        }
        // coin2pc
        if self.mint_b == WSOL_MINT {
            return false;
        }

        true
    }

    pub fn is_meteora_dlmm_buy(&self, swap_for_y: bool) -> bool {
        if swap_for_y {
            if self.mint_a == WSOL_MINT {
                return true;
            }
            return false;
        }

        if self.mint_a == WSOL_MINT {
            return false;
        }

        true
    }

    pub fn token_decimals(&self) -> u8 {
        if self.mint_a == WSOL_MINT {
            return self.decimals_b;
        }

        self.decimals_a
    }
    pub fn token_mint(&self) -> Pubkey {
        if self.mint_a == WSOL_MINT {
            return self.mint_b;
        }

        self.mint_a
    }
}

impl RedisCacheRecord for DexPoolRecord {
    fn key(&self) -> String {
        format!("{}{}", Self::prefix(), self.addr)
    }

    fn prefix() -> &'static str {
        "pool:"
    }
}
