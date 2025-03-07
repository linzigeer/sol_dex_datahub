use std::time::Instant;

use axum::extract::State;
use tracing::{debug, info};

use crate::{
    cache,
    web::{WebAppContext, WebAppError},
};

pub async fn sol_dex_stream(
    State(WebAppContext { redis_client, .. }): State<WebAppContext>,
    req_body: String,
) -> Result<(), WebAppError> {
    let start = Instant::now();
    let body_len = req_body.len();
    let mut body_start_len = 50;
    if body_len < 50 {
        body_start_len = body_len;
    }
    let body_start = &req_body[0..body_start_len];
    debug!("request body is start with: {}", body_start);
    if body_start.contains("blkTs") {
        let mut conn = redis_client.get_multiplexed_async_connection().await?;
        cache::rpush_qn_request(&mut conn, req_body).await?;
    }
    let elapsed = start.elapsed().as_millis();
    info!("process qn request take {elapsed} ms");

    Ok(())
}
