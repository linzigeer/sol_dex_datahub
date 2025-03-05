use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{MySql, MySqlConnection, QueryBuilder};

#[derive(Debug, sqlx::FromRow)]
pub struct DexPoolRow {
    pub addr: String,
    pub dex: String,
    pub mint_a: String,
    pub mint_b: String,
    pub decimals_a: u8,
    pub decimals_b: u8,
    pub created_at: DateTime<Utc>,
}

impl DexPoolRow {
    pub async fn from_db(addr: String, conn: &mut MySqlConnection) -> Result<Option<Self>> {
        let sql = "select * from pools where addr=?";

        let pool = sqlx::query_as(sql).bind(addr).fetch_optional(conn).await?;

        Ok(pool)
    }

    pub async fn batch_save(rows: &[Self], conn: &mut MySqlConnection) -> Result<()> {
        let mut qb: QueryBuilder<MySql> = QueryBuilder::new(
            "insert ignore into pools(addr, dex, mint_a, mint_b, decimals_a, decimals_b) ",
        );

        qb.push_values(rows, |mut b, row| {
            b.push_bind(&row.addr)
                .push_bind(&row.dex)
                .push_bind(&row.mint_a)
                .push_bind(&row.mint_b)
                .push_bind(row.decimals_a)
                .push_bind(row.decimals_b);
        });

        let query = qb.build();
        query.execute(conn).await?;

        Ok(())
    }
}
