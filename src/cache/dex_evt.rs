use anyhow::{Result, anyhow};
use redis::aio::MultiplexedConnection;
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::{DexPoolRecord, TradeRecord};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DexEvent {
    Trade(TradeRecord),
    PoolCreated(DexPoolRecord),
}

const DEX_EVENT_LIST_KEY: &str = "list:dex_events";
const MAX_EVENT_LEN: u64 = 200_000;
pub async fn rpush_dex_evts(conn: &mut MultiplexedConnection, events: &[DexEvent]) -> Result<()> {
    let q_len: u64 = redis::cmd("llen")
        .arg(DEX_EVENT_LIST_KEY)
        .query_async(conn)
        .await?;
    if q_len > MAX_EVENT_LEN {
        warn!("trade queue larger than 200000");
        return Err(anyhow!("trade queue larger than 200000"));
    }

    // redis rpush
    let mut cmd = redis::cmd("rpush");
    cmd.arg(DEX_EVENT_LIST_KEY);
    for evt in events {
        let json = serde_json::to_string(evt)?;
        cmd.arg(json);
    }

    let _: () = cmd.query_async(conn).await?;
    Ok(())
}

pub async fn take_dex_evts(conn: &mut MultiplexedConnection) -> Result<Vec<DexEvent>> {
    let llen: u64 = redis::cmd("llen")
        .arg(DEX_EVENT_LIST_KEY)
        .query_async(conn)
        .await?;
    if llen == 0 {
        return Ok(vec![]);
    }
    let records: Vec<String> = redis::cmd("lrange")
        .arg(DEX_EVENT_LIST_KEY)
        .arg(0)
        .arg(llen - 1)
        .query_async(conn)
        .await?;

    let mut evts = vec![];
    for record in &records {
        let evt = serde_json::from_str(record).map_err(|err| {
            anyhow!("error parse event record from redis: {err}, record: {record}")
        })?;
        evts.push(evt);
    }

    let _: () = redis::cmd("ltrim")
        .arg(DEX_EVENT_LIST_KEY)
        .arg(llen)
        .arg(-1)
        .query_async(conn)
        .await?;

    Ok(evts)
}

#[cfg(test)]
mod test {
    use chrono::Utc;
    use solana_sdk::pubkey::Pubkey;

    use crate::{
        cache::DexPoolRecord,
        common::{Dex, WSOL_MINT},
        pumpfun::PUMPFUN_PROGRAM_ID,
        raydium::RAYDIUM_AMM_PROGRAM_ID,
    };

    use super::{DexEvent, TradeRecord};

    #[test]
    fn serialize_dex_evt() {
        let evt = DexEvent::Trade(TradeRecord {
            blk_ts: Utc::now(),
            slot: 0,
            txid: "hello".to_string(),
            idx: 1,
            trader: Pubkey::default(),
            mint: WSOL_MINT,
            pool: PUMPFUN_PROGRAM_ID,
            pool_sol_amt: 100,
            pool_token_amt: 10000,
            decimals: 6,
            dex: Dex::MeteoraDlmm,
            is_buy: false,
            sol_amt: 123123,
            token_amt: 456456,
            price_sol: 0.22222,
        });
        println!("trade evt: {}", serde_json::to_string(&evt).unwrap());
        let v = serde_json::to_value(&evt).unwrap();
        assert_eq!(v.get("kind").and_then(|it| it.as_str()), Some("Trade"));

        let evt = DexEvent::PoolCreated(DexPoolRecord {
            addr: WSOL_MINT,
            dex: Dex::Pumpfun,
            mint_a: WSOL_MINT,
            mint_b: RAYDIUM_AMM_PROGRAM_ID,
            decimals_a: 9,
            decimals_b: 6,
            is_complete: false,
        });
        println!("pool created evt: {}", serde_json::to_string(&evt).unwrap());
        let v = serde_json::to_value(&evt).unwrap();
        assert_eq!(
            v.get("kind").and_then(|it| it.as_str()),
            Some("PoolCreated")
        );
    }
}
