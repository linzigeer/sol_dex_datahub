use anyhow::{Result, anyhow};
use redis::aio::MultiplexedConnection;
use tracing::warn;

const QN_REQ_LIST_KEY: &str = "list:qn_requests";
const MAX_QN_REQ_LEN: u64 = 50;
pub async fn rpush_qn_request(conn: &mut MultiplexedConnection, req: String) -> Result<()> {
    let q_len: u64 = redis::cmd("llen")
        .arg(QN_REQ_LIST_KEY)
        .query_async(conn)
        .await?;
    if q_len >= MAX_QN_REQ_LEN {
        warn!("qn request queue larger than {MAX_QN_REQ_LEN}");
        return Err(anyhow!("qn request queue larger than {MAX_QN_REQ_LEN}"));
    }

    // redis rpush
    let mut cmd = redis::cmd("rpush");
    cmd.arg(QN_REQ_LIST_KEY);
    cmd.arg(req);

    let _: () = cmd.query_async(conn).await?;
    Ok(())
}
pub async fn lrange_qn_requests(conn: &mut MultiplexedConnection) -> Result<Vec<String>> {
    let llen: u64 = redis::cmd("llen")
        .arg(QN_REQ_LIST_KEY)
        .query_async(conn)
        .await?;
    if llen == 0 {
        return Ok(vec![]);
    }
    let records: Vec<String> = redis::cmd("lrange")
        .arg(QN_REQ_LIST_KEY)
        .arg(0)
        .arg(llen - 1)
        .query_async(conn)
        .await?;
    Ok(records)
}

pub async fn ltrim_qn_requests(conn: &mut MultiplexedConnection, len: usize) -> Result<()> {
    let _: () = redis::cmd("ltrim")
        .arg(QN_REQ_LIST_KEY)
        .arg(len)
        .arg(-1)
        .query_async(conn)
        .await?;
    Ok(())
}
