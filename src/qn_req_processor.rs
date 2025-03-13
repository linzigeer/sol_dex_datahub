use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use futures::{StreamExt, TryStreamExt};
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use tracing::{info, warn};

use crate::{
    cache::{
        self, DexEvent, DexPoolCreatedRecord, DexPoolRecord, PumpfunCompleteRecord,
        RedisCacheRecord, TradeRecord,
    },
    common::TxBaseMetaInfo,
    meteora::{METEORA_DLMM_PROGRAM_ID, event::MeteoraDlmmEvents},
    pumpfun::{PUMPFUN_PROGRAM_ID, event::PumpFunEvents},
    raydium::{RAYDIUM_AMM_PROGRAM_ID, event::RayLogs},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tx {
    pub blk_ts: i64,
    pub slot: u64,
    pub signature: String,
    pub logs: Vec<String>,
    pub ixs: Vec<ProgramInvocation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramInvocation {
    pub program_id: String,
    pub instruction: Instruction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IxAccount {
    pub pubkey: String,
    pub post_amt: PostAmt,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostAmt {
    pub sol: u64,
    pub token: Option<TokenAmt>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenAmt {
    pub mint: String,
    pub decimals: u8,
    #[serde_as(as = "DisplayFromStr")]
    pub amt: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    pub accounts: Vec<IxAccount>,
    pub data: String,
    pub index: u64,
}

#[derive(Debug, Deserialize)]
pub struct QnStreamMetadata {
    pub batch_end_range: u64,
    pub batch_start_range: u64,
    pub dataset: String,
    // -1 means never end
    pub end_range: i64,
    pub keep_distance_from_tip: u64,
    pub network: String,
    pub start_range: u64,
    pub stream_id: String,
    pub stream_name: String,
    pub stream_region: String,
}

#[derive(Debug, Deserialize)]
pub struct QnSolDexDatahubWebhookReq {
    pub txs: Vec<Tx>,
    pub metadata: QnStreamMetadata,
}

pub async fn start(redis_client: Arc<redis::Client>) -> Result<()> {
    info!("start qn request processor........");
    loop {
        let start = Instant::now();
        let mut conn = redis_client.get_multiplexed_async_connection().await?;
        let reqs = cache::take_qn_requests(&mut conn).await?;
        drop(conn);

        let webhook_reqs: Vec<_> = futures::stream::iter(reqs)
            .map(|it| async move { serde_json::from_str::<QnSolDexDatahubWebhookReq>(&it) })
            .buffered(5)
            .try_collect::<Vec<_>>()
            .await?;

        let (metas, txs): (Vec<_>, Vec<_>) = webhook_reqs
            .into_iter()
            .map(|it| (it.metadata, it.txs))
            .unzip();
        for meta in metas {
            info!(
                "process slot range: [{} - {}] {} transactions from stream region: {}",
                meta.batch_start_range, meta.batch_end_range, meta.network, meta.stream_region
            );
        }

        let txs: Vec<_> = txs.into_iter().flatten().collect();
        if txs.is_empty() {
            tokio::time::sleep(Duration::from_millis(300)).await;
            continue;
        }

        let max_blk_ts = txs.iter().map(|it| it.blk_ts).max().unwrap_or_default();
        let time_diff = Utc::now().timestamp() - max_blk_ts;
        let slots: Vec<_> = txs.iter().map(|it| it.slot).collect();
        let min_slot = slots.iter().min().copied().unwrap_or_default();
        let max_slot = slots.iter().max().copied().unwrap_or_default();
        let mut all_events = vec![];
        let mut mints = HashSet::new();

        for tx in txs {
            let slot = tx.slot;
            let txid = tx.signature;
            let blk_ts = DateTime::from_timestamp(tx.blk_ts, 0)
                .ok_or_else(|| anyhow!("block timestamp error in quicknode stream"))?;
            for (idx, log) in tx.logs.into_iter().enumerate() {
                let invocation = tx.ixs.get(idx);
                if invocation.is_none() {
                    continue;
                }
                let invocation = invocation.unwrap();
                let accounts = &invocation.instruction.accounts;

                let tx_meta = TxBaseMetaInfo {
                    blk_ts,
                    slot,
                    txid: txid.clone(),
                    idx: invocation.instruction.index,
                };

                if invocation.program_id == RAYDIUM_AMM_PROGRAM_ID.to_string() {
                    match RayLogs::decode(&log.replace("Program log: ray_log: ", "")) {
                        Ok(RayLogs::Init(evt)) => {
                            // example tx: 5SPKmhBHCBphyVietx4yu3FyJ7odwLDqv5UD2sGCJpGfQu8oiVtMxiKtCvecS91G3th4nbiZz1APa8TMLncbbD6Z
                            let pool_created_record = DexPoolCreatedRecord::from_raydium_init_log(
                                tx_meta.clone(),
                                evt,
                                accounts,
                            )?;
                            let pool_record: DexPoolRecord = pool_created_record.as_pool_record();
                            let mut redis_conn =
                                redis_client.get_multiplexed_async_connection().await?;
                            pool_record.save_ex(&mut redis_conn, 3600 * 12).await?;
                            drop(redis_conn);

                            if pool_created_record.is_wsol_pool() {
                                mints.insert(pool_created_record.mint_a);
                                mints.insert(pool_created_record.mint_b);
                                all_events.push(DexEvent::PoolCreated(pool_created_record));
                            }
                        }
                        Ok(RayLogs::SwapBaseIn(evt)) => {
                            let trade = TradeRecord::from_raydium_amm_swap_base_in(
                                blk_ts,
                                slot,
                                txid.clone(),
                                invocation.instruction.index,
                                evt,
                                accounts,
                                redis_client.clone(),
                            )
                            .await?;
                            if let Some(trade) = trade {
                                mints.insert(trade.mint);
                                all_events.push(DexEvent::Trade(trade));
                            }
                        }
                        Ok(RayLogs::SwapBaseOut(evt)) => {
                            let trade = TradeRecord::from_raydium_amm_swap_base_out(
                                blk_ts,
                                slot,
                                txid.clone(),
                                invocation.instruction.index,
                                evt,
                                accounts,
                                redis_client.clone(),
                            )
                            .await?;
                            if let Some(trade) = trade {
                                mints.insert(trade.mint);
                                all_events.push(DexEvent::Trade(trade));
                            }
                        }
                        Err(err) => {
                            warn!("!!!!!!!!!!!!! parse ray amm log error: {err}, tx: {txid}");
                            continue;
                        }
                        _ => continue,
                    }
                } else if invocation.program_id == PUMPFUN_PROGRAM_ID.to_string() {
                    match PumpFunEvents::from_cpi_log(&log.replace("pumpfun cpi log: ", "")) {
                        Ok(PumpFunEvents::Create(evt)) => {
                            let pool_created_record =
                                DexPoolCreatedRecord::from_pumpfun_create_log(tx_meta.clone(), evt);

                            let pool_record = pool_created_record.as_pool_record();
                            let mut redis_conn =
                                redis_client.get_multiplexed_async_connection().await?;
                            pool_record.save_ex(&mut redis_conn, 3600 * 12).await?;
                            drop(redis_conn);

                            if pool_created_record.is_wsol_pool() {
                                mints.insert(pool_created_record.mint_a);
                                mints.insert(pool_created_record.mint_b);
                                all_events.push(DexEvent::PoolCreated(pool_created_record));
                            }
                        }
                        Ok(PumpFunEvents::Trade(evt)) => {
                            let trade = TradeRecord::from_pumpfun_trade(
                                blk_ts,
                                slot,
                                txid.clone(),
                                invocation.instruction.index,
                                evt,
                                accounts,
                                redis_client.clone(),
                            )
                            .await?;
                            if let Some(trade) = trade {
                                mints.insert(trade.mint);
                                all_events.push(DexEvent::Trade(trade));
                            }
                        }
                        Ok(PumpFunEvents::Complete(evt)) => {
                            let pool_record = DexPoolRecord::from_pumpfun_curve_and_mint(
                                evt.bonding_curve,
                                evt.mint,
                                true,
                            );
                            let mut redis_conn =
                                redis_client.get_multiplexed_async_connection().await?;
                            pool_record.save_ex(&mut redis_conn, 3600 * 12).await?;
                            drop(redis_conn);

                            let complete_evt = PumpfunCompleteRecord::new(tx_meta.clone(), &evt);
                            mints.insert(complete_evt.mint);
                            all_events.push(DexEvent::PumpfunComplete(complete_evt))
                        }
                        Err(err) => {
                            warn!("!!!!!!!!!!!!! parse pumpfun log error: {err}, tx: {txid}");
                            continue;
                        }
                        _ => continue,
                    }
                } else if invocation.program_id == METEORA_DLMM_PROGRAM_ID.to_string() {
                    match MeteoraDlmmEvents::from_cpi_log(
                        &log.replace("meteora dlmm cpi log: ", ""),
                    ) {
                        Ok(MeteoraDlmmEvents::LbPairCreate(evt)) => {
                            let pool_created_record =
                                DexPoolCreatedRecord::from_meteora_dlmm_lp_create_log(
                                    tx_meta.clone(),
                                    evt,
                                    accounts,
                                )?;
                            let pool_record: DexPoolRecord = pool_created_record.as_pool_record();
                            let mut redis_conn =
                                redis_client.get_multiplexed_async_connection().await?;
                            pool_record.save_ex(&mut redis_conn, 3600 * 12).await?;
                            drop(redis_conn);

                            if pool_created_record.is_wsol_pool() {
                                mints.insert(pool_created_record.mint_a);
                                mints.insert(pool_created_record.mint_b);
                                all_events.push(DexEvent::PoolCreated(pool_created_record));
                            }
                        }
                        Ok(MeteoraDlmmEvents::Swap(evt)) => {
                            let trade = TradeRecord::from_meteora_dlmm_swap(
                                blk_ts,
                                slot,
                                txid.clone(),
                                invocation.instruction.index,
                                evt,
                                accounts,
                                redis_client.clone(),
                            )
                            .await?;
                            if let Some(trade) = trade {
                                mints.insert(trade.mint);
                                all_events.push(DexEvent::Trade(trade));
                            }
                        }
                        Err(_err) => {
                            // warn!("!!!!!!!!!!!!! parse meteora dlmm log error: {err}, tx: {txid}");
                            continue;
                        }
                    }
                }
            }
        }

        let events_len = all_events.len();
        if events_len > 0 {
            let mut conn = redis_client.get_multiplexed_async_connection().await?;
            cache::rpush_dex_evts(&mut conn, &all_events).await?;
            drop(conn);
            let ms = start.elapsed().as_millis();
            info!(
                "parsed events: {events_len}, parse take time: {ms} ms, slot range: [{min_slot} - {max_slot}] time diff: {time_diff} seconds"
            );
        }

        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}
