use std::{str::FromStr, sync::Arc};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc, serde::ts_seconds};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use crate::{
    cache::{DexPoolRecord, RedisCacheRecord},
    common::{Dex, TxBaseMetaInfo, WSOL_MINT, utils},
    meteora::{damm::event::MeteoraDammSwap, dlmm::event::MeteoraDlmmSwapEvent},
    pumpamm::event::{PumpAmmBuyEvent, PumpAmmSellEvent},
    pumpfun::event::TradeEvent,
    qn_req_processor::IxAccount,
    raydium::event::{SwapBaseInLog, SwapBaseOutLog},
};
use solana_sdk::pubkey::Pubkey;

use super::DEX_POOL_RECORD_EXP_SECS;

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct TradeRecord {
    #[serde(with = "ts_seconds")]
    pub blk_ts: DateTime<Utc>,
    pub slot: u64,
    pub txid: String,
    pub idx: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub mint: Pubkey,
    pub decimals: u8,
    #[serde_as(as = "DisplayFromStr")]
    pub trader: Pubkey,
    pub dex: Dex,
    #[serde_as(as = "DisplayFromStr")]
    pub pool: Pubkey,
    pub pool_sol_amt: u64,
    pub pool_token_amt: u64,
    pub is_buy: bool,
    pub sol_amt: u64,
    pub token_amt: u64,
    pub price_sol: f64,
}

impl TradeRecord {
    pub async fn from_pumpamm_buy(
        TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        }: TxBaseMetaInfo,
        log: PumpAmmBuyEvent,
        accounts: &[IxAccount],
        redis_client: Arc<redis::Client>,
    ) -> Result<Option<Self>> {
        let pool = log.pool;
        let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
        let cached_pool =
            DexPoolRecord::from_pumpamm_swap_accounts(pool, accounts, &mut redis_conn).await?;
        cached_pool
            .save_ex(&mut redis_conn, DEX_POOL_RECORD_EXP_SECS)
            .await?;
        drop(redis_conn);
        if !cached_pool.is_wsol_pool() {
            // only accept WSOL pair
            return Ok(None);
        }

        let base_token_vault = accounts
            .get(7)
            .ok_or_else(|| anyhow!("need base token vault in pumpamm swap log"))?;
        let base_token_amt = base_token_vault
            .post_amt
            .token
            .clone()
            .ok_or_else(|| anyhow!("base token should have balance in pumpamm swap log"))?;

        let quote_token_vault = accounts
            .get(8)
            .ok_or_else(|| anyhow!("need quote token vault in pumpamm swap log"))?;
        let quote_token_amt = quote_token_vault
            .post_amt
            .token
            .clone()
            .ok_or_else(|| anyhow!("quote token should have balance in pumpamm swap log"))?;

        let (pool_sol_amt, pool_token_amt, sol_amt, token_amt, is_buy) =
            if cached_pool.mint_a == WSOL_MINT {
                (
                    base_token_amt.amt,
                    quote_token_amt.amt,
                    log.base_amount_out,
                    log.quote_amount_in_with_lp_fee,
                    false,
                )
            } else {
                (
                    quote_token_amt.amt,
                    base_token_amt.amt,
                    log.quote_amount_in_with_lp_fee,
                    log.base_amount_out,
                    true,
                )
            };

        let trader = log.user;
        let mint = cached_pool.token_mint();
        let decimals = cached_pool.token_decimals();
        let price_sol = utils::calc_price_sol(sol_amt, token_amt, decimals);

        Ok(Some(Self {
            blk_ts,
            slot,
            txid,
            idx,
            mint,
            decimals,
            trader,
            dex: Dex::PumpAmm,
            pool,
            pool_token_amt,
            pool_sol_amt,
            is_buy,
            sol_amt,
            token_amt,
            price_sol,
        }))
    }

    pub async fn from_pumpamm_sell(
        TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        }: TxBaseMetaInfo,
        log: PumpAmmSellEvent,
        accounts: &[IxAccount],
        redis_client: Arc<redis::Client>,
    ) -> Result<Option<Self>> {
        let pool = log.pool;
        let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
        let cached_pool =
            DexPoolRecord::from_pumpamm_swap_accounts(pool, accounts, &mut redis_conn).await?;
        cached_pool
            .save_ex(&mut redis_conn, DEX_POOL_RECORD_EXP_SECS)
            .await?;
        drop(redis_conn);
        if !cached_pool.is_wsol_pool() {
            // only accept WSOL pair
            return Ok(None);
        }

        let base_token_vault = accounts
            .get(7)
            .ok_or_else(|| anyhow!("need base token vault in pumpamm swap log"))?;
        let base_token_amt = base_token_vault
            .post_amt
            .token
            .clone()
            .ok_or_else(|| anyhow!("base token should have balance in pumpamm swap log"))?;

        let quote_token_vault = accounts
            .get(8)
            .ok_or_else(|| anyhow!("need quote token vault in pumpamm swap log"))?;
        let quote_token_amt = quote_token_vault
            .post_amt
            .token
            .clone()
            .ok_or_else(|| anyhow!("quote token should have balance in pumpamm swap log"))?;

        let (pool_sol_amt, pool_token_amt, sol_amt, token_amt, is_buy) =
            if cached_pool.mint_a == WSOL_MINT {
                (
                    base_token_amt.amt,
                    quote_token_amt.amt,
                    log.base_amount_in,
                    log.user_quote_amount_out,
                    true,
                )
            } else {
                (
                    quote_token_amt.amt,
                    base_token_amt.amt,
                    log.user_quote_amount_out,
                    log.base_amount_in,
                    false,
                )
            };

        let trader = log.user;
        let mint = cached_pool.token_mint();
        let decimals = cached_pool.token_decimals();
        let price_sol = utils::calc_price_sol(sol_amt, token_amt, decimals);

        Ok(Some(Self {
            blk_ts,
            slot,
            txid,
            idx,
            mint,
            decimals,
            trader,
            dex: Dex::PumpAmm,
            pool,
            pool_token_amt,
            pool_sol_amt,
            is_buy,
            sol_amt,
            token_amt,
            price_sol,
        }))
    }

    pub async fn from_meteora_dlmm_swap(
        TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        }: TxBaseMetaInfo,
        log: MeteoraDlmmSwapEvent,
        accounts: &[IxAccount],
        redis_client: Arc<redis::Client>,
    ) -> Result<Option<Self>> {
        let pool_acc = accounts
            .first()
            .ok_or_else(|| anyhow!("need meteora dlmm lbpair pubkey in swap log"))?;
        let lb_pair_pubkey = Pubkey::from_str(&pool_acc.pubkey)?;
        let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
        let cached_pool =
            DexPoolRecord::from_meteora_swap_accounts(lb_pair_pubkey, accounts, &mut redis_conn)
                .await?;
        cached_pool
            .save_ex(&mut redis_conn, DEX_POOL_RECORD_EXP_SECS)
            .await?;
        drop(redis_conn);
        if !cached_pool.is_wsol_pool() {
            // only accept WSOL pair
            return Ok(None);
        }

        let trader_acc = accounts
            .get(10)
            .ok_or_else(|| anyhow!("need trader pubkey in meteora dlmm swap log"))?;
        let trader = Pubkey::from_str(&trader_acc.pubkey)?;

        let token_x_vault = accounts
            .get(2)
            .ok_or_else(|| anyhow!("need token x value in meteora dlmm swap log"))?;
        let pool_token_x_amt = token_x_vault.post_amt.token.clone().ok_or_else(|| {
            anyhow!(
                "meteora dlmm token x vault {} should have balance",
                token_x_vault.pubkey
            )
        })?;
        let token_y_vault = accounts
            .get(3)
            .ok_or_else(|| anyhow!("need token y value in meteora dlmm swap log"))?;
        let pool_token_y_amt = token_y_vault.post_amt.token.clone().ok_or_else(|| {
            anyhow!(
                "meteora dlmm token y vault {} should have balance",
                token_y_vault.pubkey
            )
        })?;
        let is_token_x_sol = pool_token_x_amt.mint == WSOL_MINT.to_string();

        let is_buy = cached_pool.is_meteora_dlmm_buy(log.swap_for_y);
        let sol_amt = if log.swap_for_y {
            if cached_pool.mint_a == WSOL_MINT {
                log.amount_in
            } else {
                log.amount_out
            }
        } else if cached_pool.mint_a == WSOL_MINT {
            log.amount_out
        } else {
            log.amount_in
        };
        let token_amt = if log.swap_for_y {
            if cached_pool.mint_a == WSOL_MINT {
                log.amount_out
            } else {
                log.amount_in
            }
        } else if cached_pool.mint_a == WSOL_MINT {
            log.amount_in
        } else {
            log.amount_out
        };
        if sol_amt == 0 || token_amt == 0 {
            return Ok(None);
        }

        let mint = cached_pool.token_mint();
        let decimals = cached_pool.token_decimals();
        let price_sol = utils::calc_price_sol(sol_amt, token_amt, decimals);

        let (pool_token_amt, pool_sol_amt) = if is_token_x_sol {
            (pool_token_y_amt.amt, pool_token_x_amt.amt)
        } else {
            (pool_token_x_amt.amt, pool_token_y_amt.amt)
        };

        Ok(Some(Self {
            blk_ts,
            slot,
            txid,
            idx,
            mint,
            decimals,
            trader,
            dex: Dex::MeteoraDlmm,
            pool: lb_pair_pubkey,
            pool_token_amt,
            pool_sol_amt,
            is_buy,
            sol_amt,
            token_amt,
            price_sol,
        }))
    }

    pub async fn from_meteora_damm_swap(
        TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        }: TxBaseMetaInfo,
        log: MeteoraDammSwap,
        accounts: &[IxAccount],
        redis_client: Arc<redis::Client>,
    ) -> Result<Option<Self>> {
        let pool_acc = accounts
            .first()
            .ok_or_else(|| anyhow!("need meteora damm pool pubkey in swap log"))?;
        let pool_pubkey = Pubkey::from_str(&pool_acc.pubkey)?;
        let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
        let cached_pool =
            DexPoolRecord::from_meteora_damm_swap_accounts(pool_pubkey, accounts, &mut redis_conn)
                .await?;
        cached_pool
            .save_ex(&mut redis_conn, DEX_POOL_RECORD_EXP_SECS)
            .await?;
        drop(redis_conn);
        if !cached_pool.is_wsol_pool() {
            // only accept WSOL pair
            return Ok(None);
        }

        let trader_acc = accounts
            .get(12)
            .ok_or_else(|| anyhow!("need trader pubkey in meteora damm swap log"))?;
        let trader = Pubkey::from_str(&trader_acc.pubkey)?;

        let token_a_vault = accounts
            .get(5)
            .ok_or_else(|| anyhow!("need token x value in meteora damm swap log"))?;
        let pool_token_a_amt = token_a_vault.post_amt.token.clone().ok_or_else(|| {
            anyhow!(
                "meteora damm token a vault {} should have balance",
                token_a_vault.pubkey
            )
        })?;
        let token_b_vault = accounts
            .get(6)
            .ok_or_else(|| anyhow!("need token b value in meteora damm swap log"))?;
        let pool_token_b_amt = token_b_vault.post_amt.token.clone().ok_or_else(|| {
            anyhow!(
                "meteora damm token b vault {} should have balance",
                token_b_vault.pubkey
            )
        })?;

        let user_source_token_mint = accounts
            .get(1)
            .and_then(|it| {
                it.pre_amt
                    .token
                    .clone()
                    .or_else(|| it.post_amt.token.clone())
            })
            .map(|it| it.mint);
        let user_dest_token_mint = accounts
            .get(2)
            .and_then(|it| {
                it.pre_amt
                    .token
                    .clone()
                    .or_else(|| it.post_amt.token.clone())
            })
            .map(|it| it.mint);

        if user_source_token_mint.is_none() && user_dest_token_mint.is_none() {
            anyhow::bail!(
                "meteora damm swap have no user source and destination token balance change"
            );
        }

        let is_buy = if let Some(user_source_token_mint) = user_source_token_mint {
            user_source_token_mint == WSOL_MINT.to_string()
        } else {
            user_dest_token_mint.unwrap() != WSOL_MINT.to_string()
        };
        let (sol_amt, token_amt) = if is_buy {
            (log.in_amount - log.protocol_fee, log.out_amount)
        } else {
            (log.out_amount, log.in_amount - log.protocol_fee)
        };
        if sol_amt == 0 || token_amt == 0 {
            return Ok(None);
        }

        let mint = cached_pool.token_mint();
        let decimals = cached_pool.token_decimals();
        let price_sol = utils::calc_price_sol(sol_amt, token_amt, decimals);

        let is_token_a_sol = pool_token_a_amt.mint == WSOL_MINT.to_string();
        let (pool_token_amt, pool_sol_amt) = if is_token_a_sol {
            (pool_token_b_amt.amt, pool_token_a_amt.amt)
        } else {
            (pool_token_a_amt.amt, pool_token_b_amt.amt)
        };

        Ok(Some(Self {
            blk_ts,
            slot,
            txid,
            idx,
            mint,
            decimals,
            trader,
            dex: Dex::MeteoraDamm,
            pool: pool_pubkey,
            pool_token_amt,
            pool_sol_amt,
            is_buy,
            sol_amt,
            token_amt,
            price_sol,
        }))
    }

    pub async fn from_raydium_amm_swap_base_in(
        TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        }: TxBaseMetaInfo,
        log: SwapBaseInLog,
        accounts: &[IxAccount],
        redis_client: Arc<redis::Client>,
    ) -> Result<Option<Self>> {
        let pool_acc = accounts
            .get(1)
            .ok_or_else(|| anyhow!("need amm pubkey in swap base in log"))?;
        let amm_pubkey = Pubkey::from_str(&pool_acc.pubkey)?;
        let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
        let cached_pool =
            DexPoolRecord::from_raydim_amm_trade_accounts(amm_pubkey, accounts, &mut redis_conn)
                .await?;
        cached_pool
            .save_ex(&mut redis_conn, DEX_POOL_RECORD_EXP_SECS)
            .await?;
        drop(redis_conn);

        if !cached_pool.is_wsol_pool() {
            // only accept WSOL pair
            return Ok(None);
        }

        // example tx: 3JwTJ11gDVicXmyjGoemuy3NP7zypiq3FvWQWyR99wdi3iRcrhf3kcEwszpjn5P8MX5uiKLYKr8HnegPynR6mL4y
        let trader_acc = accounts
            .last()
            .ok_or_else(|| anyhow!("need trader pubkey in swap base in log"))?;
        let trader = Pubkey::from_str(&trader_acc.pubkey)?;

        let mut coin_token_vault_idx = 4;
        let mut pc_token_vault_idx = 5;
        if accounts.len() == 18 {
            coin_token_vault_idx = 5;
            pc_token_vault_idx = 6;
        }

        let coin_token_vault = accounts
            .get(coin_token_vault_idx)
            .ok_or_else(|| anyhow!("need coin token vault in raydium amm swap base in log"))?;
        let coin_token_amt =
            coin_token_vault.post_amt.token.clone().ok_or_else(|| {
                anyhow!("coin token should have balance in raydium amm base in swap")
            })?;
        let pc_token_vault = accounts
            .get(pc_token_vault_idx)
            .ok_or_else(|| anyhow!("need pc token vault in raydium amm swap base in log"))?;
        let pc_token_amt = pc_token_vault.post_amt.token.clone().ok_or_else(|| {
            anyhow!("pc token should have balance in raydium amm base in swap in txid: {txid}")
        })?;
        let is_coin_token_sol = coin_token_amt.mint == WSOL_MINT.to_string();

        let is_buy = cached_pool.is_raydium_buy(log.direction);
        let sol_amt = if log.direction == 1 {
            // pc2coin
            if cached_pool.mint_b == WSOL_MINT {
                log.amount_in
            } else {
                log.out_amount
            }
        } else {
            // coin2pc
            if cached_pool.mint_b == WSOL_MINT {
                log.out_amount
            } else {
                log.amount_in
            }
        };
        let token_amt = if log.direction == 1 {
            // pc2coin
            if cached_pool.mint_b == WSOL_MINT {
                log.out_amount
            } else {
                log.amount_in
            }
        } else {
            // coin2pc
            if cached_pool.mint_b == WSOL_MINT {
                log.amount_in
            } else {
                log.out_amount
            }
        };
        if sol_amt == 0 || token_amt == 0 {
            return Ok(None);
        }

        let mint = cached_pool.token_mint();
        let decimals = cached_pool.token_decimals();
        let price_sol = utils::calc_price_sol(sol_amt, token_amt, decimals);

        let (pool_token_amt, pool_sol_amt) = if is_coin_token_sol {
            (pc_token_amt.amt, coin_token_amt.amt)
        } else {
            (coin_token_amt.amt, pc_token_amt.amt)
        };

        Ok(Some(Self {
            blk_ts,
            slot,
            txid,
            idx,
            mint,
            decimals,
            trader,
            dex: Dex::RaydiumAmm,
            pool: amm_pubkey,
            pool_sol_amt,
            pool_token_amt,
            is_buy,
            sol_amt,
            token_amt,
            price_sol,
        }))
    }

    pub async fn from_raydium_amm_swap_base_out(
        TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        }: TxBaseMetaInfo,
        log: SwapBaseOutLog,
        accounts: &[IxAccount],
        redis_client: Arc<redis::Client>,
    ) -> Result<Option<Self>> {
        let pool_acc = accounts
            .get(1)
            .ok_or_else(|| anyhow!("need amm pubkey in swap base out log"))?;
        let amm_pubkey = Pubkey::from_str(&pool_acc.pubkey)?;
        let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
        let cached_pool =
            DexPoolRecord::from_raydim_amm_trade_accounts(amm_pubkey, accounts, &mut redis_conn)
                .await?;
        cached_pool
            .save_ex(&mut redis_conn, DEX_POOL_RECORD_EXP_SECS)
            .await?;
        drop(redis_conn);

        if !cached_pool.is_wsol_pool() {
            // only accept WSOL pair
            return Ok(None);
        }

        // example tx: 2ff5Kxnu2V2Pa7TEsvJ9aDQF6VWYWiB9zR954PszxRNg52kiXavYU7AAUaCcEsGYU9GU7mHRYuSdjHvXege5dGWM
        let trader_acc = accounts
            .last()
            .ok_or_else(|| anyhow!("need trader pubkey in swap base out log"))?;
        let trader = Pubkey::from_str(&trader_acc.pubkey)?;

        let mut coin_token_vault_idx = 4;
        let mut pc_token_vault_idx = 5;
        if accounts.len() == 18 {
            coin_token_vault_idx = 5;
            pc_token_vault_idx = 6;
        }

        let coin_token_vault = accounts
            .get(coin_token_vault_idx)
            .ok_or_else(|| anyhow!("need coin token vault in raydium amm swap base out log"))?;
        let coin_token_amt = coin_token_vault.post_amt.token.clone().ok_or_else(|| {
            anyhow!("coin token should have balance in raydium amm base out swap")
        })?;
        let pc_token_vault = accounts
            .get(pc_token_vault_idx)
            .ok_or_else(|| anyhow!("need pc token vault in raydium amm swap base out log"))?;
        let pc_token_amt =
            pc_token_vault.post_amt.token.clone().ok_or_else(|| {
                anyhow!("pc token should have balance in raydium amm base out swap")
            })?;
        let is_coin_token_sol = coin_token_amt.mint == WSOL_MINT.to_string();

        let is_buy = cached_pool.is_raydium_buy(log.direction);
        let sol_amt = if log.direction == 1 {
            // pc2coin
            if cached_pool.mint_b == WSOL_MINT {
                log.deduct_in
            } else {
                log.amount_out
            }
        } else {
            // coin2pc
            if cached_pool.mint_b == WSOL_MINT {
                log.amount_out
            } else {
                log.deduct_in
            }
        };
        let token_amt = if log.direction == 1 {
            // pc2coin
            if cached_pool.mint_b == WSOL_MINT {
                log.amount_out
            } else {
                log.deduct_in
            }
        } else {
            // coin2pc
            if cached_pool.mint_b == WSOL_MINT {
                log.deduct_in
            } else {
                log.amount_out
            }
        };
        if sol_amt == 0 || token_amt == 0 {
            return Ok(None);
        }

        let mint = cached_pool.token_mint();
        let decimals = cached_pool.token_decimals();
        let price_sol = utils::calc_price_sol(sol_amt, token_amt, decimals);

        let (pool_token_amt, pool_sol_amt) = if is_coin_token_sol {
            (pc_token_amt.amt, coin_token_amt.amt)
        } else {
            (coin_token_amt.amt, pc_token_amt.amt)
        };

        Ok(Some(Self {
            blk_ts,
            slot,
            txid,
            idx,
            mint,
            decimals,
            trader,
            dex: Dex::RaydiumAmm,
            pool: amm_pubkey,
            pool_sol_amt,
            pool_token_amt,
            is_buy,
            sol_amt,
            token_amt,
            price_sol,
        }))
    }

    pub async fn from_pumpfun_trade(
        TxBaseMetaInfo {
            blk_ts,
            slot,
            txid,
            idx,
        }: TxBaseMetaInfo,
        log: TradeEvent,
        accounts: &[IxAccount],
        redis_client: Arc<redis::Client>,
    ) -> Result<Option<Self>> {
        let pool_acc = accounts
            .get(3)
            .ok_or_else(|| anyhow!("need curve pubkey in pumpfun trade"))?;
        let curve_pubkey = Pubkey::from_str(&pool_acc.pubkey)?;
        let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
        let cached_pool =
            DexPoolRecord::from_pumpfun_trade_accounts(accounts, &mut redis_conn).await?;
        cached_pool
            .save_ex(&mut redis_conn, DEX_POOL_RECORD_EXP_SECS)
            .await?;
        drop(redis_conn);

        if !cached_pool.is_wsol_pool() {
            // only accept WSOL pair
            return Ok(None);
        }

        let trader_acc = accounts
            .get(6)
            .ok_or_else(|| anyhow!("need trader pubkey in pumpfun trade"))?;
        let trader = Pubkey::from_str(&trader_acc.pubkey)?;
        let is_buy = log.is_buy;
        let sol_amt = log.sol_amount;
        let token_amt = log.token_amount;
        let pool_sol_amt = log.real_sol_reserves;
        let pool_token_amt = log.real_token_reserves;
        if sol_amt == 0 || token_amt == 0 {
            return Ok(None);
        }

        let mint = cached_pool.token_mint();
        let decimals = cached_pool.token_decimals();
        let price_sol = utils::calc_price_sol(sol_amt, token_amt, decimals);

        Ok(Some(Self {
            blk_ts,
            slot,
            txid,
            idx,
            mint,
            decimals,
            trader,
            dex: Dex::Pumpfun,
            pool: curve_pubkey,
            pool_sol_amt,
            pool_token_amt,
            is_buy,
            sol_amt,
            token_amt,
            price_sol,
        }))
    }
}
