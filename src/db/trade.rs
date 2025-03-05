use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{MySql, MySqlConnection, QueryBuilder};

#[derive(Debug, sqlx::FromRow)]
pub struct TradeRow {
    pub blk_ts: DateTime<Utc>,
    pub slot: u64,
    pub txid: String,
    pub idx: u64,
    pub mint: String,
    pub decimals: u8,
    pub trader: String,
    pub dex: String,
    pub pool: String,
    pub is_buy: bool,
    pub sol_amt: u64,
    pub token_amt: u64,
    pub price_sol: f64,
    pub created_at: DateTime<Utc>,
}

impl TradeRow {
    pub async fn batch_save(rows: &[Self], conn: &mut MySqlConnection) -> Result<()> {
        let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
            "insert ignore into trades(blk_ts, slot, txid, idx, mint, decimals, trader, dex, pool, is_buy, sol_amt, token_amt, price_sol) ",
        );

        qb.push_values(rows, |mut b, row| {
            b.push_bind(row.blk_ts)
                .push_bind(row.slot)
                .push_bind(&row.txid)
                .push_bind(row.idx)
                .push_bind(&row.mint)
                .push_bind(row.decimals)
                .push_bind(&row.trader)
                .push_bind(&row.dex)
                .push_bind(&row.pool)
                .push_bind(row.is_buy)
                .push_bind(row.sol_amt)
                .push_bind(row.token_amt)
                .push_bind(row.price_sol);
        });

        let query = qb.build();
        query.execute(conn).await?;

        Ok(())
    }
}
