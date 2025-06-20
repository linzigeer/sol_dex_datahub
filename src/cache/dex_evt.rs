use anyhow::{Result, anyhow};
use redis::aio::MultiplexedConnection;
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::{DexPoolCreatedRecord, PumpfunCompleteRecord, TradeRecord};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DexEvent {
    Trade(TradeRecord),
    PoolCreated(DexPoolCreatedRecord),
    PumpfunComplete(PumpfunCompleteRecord),
}

const DEX_EVENT_LIST_KEY: &str = "list:dex_events";
const MAX_EVENT_LEN: u64 = 50_000;
pub async fn rpush_dex_evts(conn: &mut MultiplexedConnection, events: &[DexEvent]) -> Result<()> {
    let q_len: u64 = redis::cmd("llen")
        .arg(DEX_EVENT_LIST_KEY)
        .query_async(conn)
        .await?;
    if q_len >= MAX_EVENT_LEN {
        warn!("trade queue larger than {MAX_EVENT_LEN}");
        return Err(anyhow!("trade queue larger than {MAX_EVENT_LEN}"));
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

pub async fn lrange_dex_evts(conn: &mut MultiplexedConnection) -> Result<Vec<DexEvent>> {
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

    Ok(evts)
}

pub async fn ltrim_dex_evts(conn: &mut MultiplexedConnection, len: usize) -> Result<()> {
    let _: () = redis::cmd("ltrim")
        .arg(DEX_EVENT_LIST_KEY)
        .arg(len)
        .arg(-1)
        .query_async(conn)
        .await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{
        cache::DexPoolCreatedRecord,
        common::{Dex, WSOL_MINT},
        pumpfun::PUMPFUN_PROGRAM_ID,
        raydium::RAYDIUM_AMM_PROGRAM_ID,
    };
    use chrono::Utc;
    use solana_sdk::pubkey::Pubkey;
    use std::any::type_name_of_val;
    use std::collections::HashMap;

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

        let evt = DexEvent::PoolCreated(DexPoolCreatedRecord {
            blk_ts: Utc::now(),
            slot: 1,
            txid: "txid123".to_string(),
            idx: 6,
            creator: RAYDIUM_AMM_PROGRAM_ID,
            addr: WSOL_MINT,
            dex: Dex::Pumpfun,
            mint_a: WSOL_MINT,
            mint_b: RAYDIUM_AMM_PROGRAM_ID,
            decimals_a: 9,
            decimals_b: 6,
        });
        println!("pool created evt: {}", serde_json::to_string(&evt).unwrap());
        let v = serde_json::to_value(&evt).unwrap();
        assert_eq!(
            v.get("kind").and_then(|it| it.as_str()),
            Some("PoolCreated")
        );

        let (a, b) = ("a", 4);
        let (mut to_be, max_int, z) = (false, 1 << 30, "a");
        to_be = true;
        let name_of_val = type_name_of_val(&to_be);
        println!("{}", name_of_val);
        println!("{}", a);
        println!("{}", b);

        for i in 0..10 {
            println!("{}", i);
        }
    }

    ///牛顿法求平方根
    #[test]
    pub fn find_sqr_of_42() {
        let x = 42f64;
        let mut z = x as f64 / 2.0;
        let mut counter = 0;

        let now = std::time::Instant::now();
        while z * z - x > 0.00000001 {
            counter += 1;
            if z * z > x {
                z -= 0.00000001f64;
            } else {
                z += 0.00000001f64;
            }
        }
        println!("time elapsed: {:?}", now.elapsed());
        println!("counter: {}", counter);
        println!("z: {}", z);
    }

    #[test]
    fn test_slice() {
        let v = vec![2, 3, 5, 7, 11, 13];
        let mut s = &v[..];
        println!("slice1: {:?}", s);
        s = &s[..4];
        println!("slice2: {:?}", s);
        s = &s[2..];
        println!("slice3: {:?}", s);

        let mut map = HashMap::new();
        map.insert("a", 1);
        println!("{}", map["a"]); // this will panic if keys doesn't exist
        println!("{:#?}", map.get("A"));

        let _option = map.remove("a");
    }

    #[test]
    fn test_wc() {
        let s = "hello, this is major tom. hello, major tom, this is your captain speaking.";
        let mut map = HashMap::new();
        let _ = s
            .split(' ')
            .into_iter()
            .for_each(|word| *map.entry(word).or_insert(0) += 1);

        println!("map:{:?}", map);
    }

    pub fn test1(f1: fn(u8, u8)->u8) -> u8 {
        f1(1, 2)
    }

    #[test]
    fn test_fn() {

    }
}




