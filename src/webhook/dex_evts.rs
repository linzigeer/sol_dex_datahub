use std::{sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use reqwest::header;
use tracing::{info, warn};

use crate::cache;

pub struct DexEvtWebhook {
    pub redis_client: Arc<redis::Client>,
    pub http_client: Arc<reqwest::Client>,
    pub endpoint: String,
}

impl DexEvtWebhook {
    pub async fn start(&self) -> Result<()> {
        loop {
            let mut conn = self.redis_client.get_multiplexed_async_connection().await?;
            let events = cache::lrange_dex_evts(&mut conn)
                .await
                .map_err(|err| anyhow!("lrange dex events error: {err}"))?;

            let events_len = events.len();
            if events_len == 0 {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
            info!("send {} trades to webhook: {}", events_len, self.endpoint);
            let msg = serde_json::to_string(&events)
                .map_err(|err| anyhow!("failed serialize dex events from redis: {err}"))?;
            let webhook_resp = self
                .http_client
                .post(&self.endpoint)
                .header(header::CONTENT_TYPE, "application/json")
                .body(msg)
                .send()
                .await
                .map_err(|err| anyhow!("send dex events to webhhook failed: {err}"))?;

            let webhook_resp_status = webhook_resp.status();
            if webhook_resp_status == reqwest::StatusCode::OK {
                cache::ltrim_dex_evts(&mut conn, events_len).await?;
            } else {
                warn!(
                    "send dex events to webhook failed, status is not 200 is: {webhook_resp_status}"
                );
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}
