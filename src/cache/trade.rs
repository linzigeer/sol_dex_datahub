use std::{str::FromStr, sync::Arc};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc, serde::ts_seconds};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

use crate::{
    cache::{DexPoolRecord, RedisCacheRecord},
    common::{Dex, WSOL_MINT, utils},
    meteora::event::MeteoraDlmmSwapEvent,
    pumpfun::event::TradeEvent,
    qn_req_processor::IxAccount,
    raydium::event::{SwapBaseInLog, SwapBaseOutLog},
};
use solana_sdk::pubkey::Pubkey;

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
    #[allow(clippy::too_many_arguments)]
    pub async fn from_meteora_dlmm_swap(
        blk_ts: DateTime<Utc>,
        slot: u64,
        txid: String,
        idx: u64,
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
        cached_pool.save_ex(&mut redis_conn, 3600 * 12).await?;
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
        let pool_token_y_amt = token_x_vault.post_amt.token.clone().ok_or_else(|| {
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

    #[allow(clippy::too_many_arguments)]
    pub async fn from_raydium_amm_swap_base_in(
        blk_ts: DateTime<Utc>,
        slot: u64,
        txid: String,
        idx: u64,
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
        cached_pool.save_ex(&mut redis_conn, 3600 * 12).await?;
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

    #[allow(clippy::too_many_arguments)]
    pub async fn from_raydium_amm_swap_base_out(
        blk_ts: DateTime<Utc>,
        slot: u64,
        txid: String,
        idx: u64,
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
        cached_pool.save_ex(&mut redis_conn, 3600 * 12).await?;
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
        blk_ts: DateTime<Utc>,
        slot: u64,
        txid: String,
        idx: u64,
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
        cached_pool.save_ex(&mut redis_conn, 3600 * 12).await?;
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
