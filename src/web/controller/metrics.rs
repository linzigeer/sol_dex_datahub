use axum::extract::State;
use redis::AsyncCommands;
use serde::Serialize;

use crate::web::{WebAppContext, WebAppError, extractor::json::Json};

#[derive(Debug, Serialize)]
pub struct MetricsResp {
    pub latest_sol_slot: u64,
    pub redis_test: String,
}

pub async fn check_health(
    State(WebAppContext {
        redis_client,
        sol_rpc_client,
        ..
    }): State<WebAppContext>,
) -> Result<Json<MetricsResp>, WebAppError> {
    let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
    let _: () = redis_conn.set_ex("check_health", b"ok", 10).await?;
    let redis_result: String = redis_conn.get("check_health").await?;
    drop(redis_conn);

    let latest_sol_slot = sol_rpc_client.get_slot().await?;

    Ok(Json(MetricsResp {
        latest_sol_slot,
        redis_test: redis_result,
    }))
}
