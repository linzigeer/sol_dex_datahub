use std::{sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use reqwest::header;
use serde::Serialize;
use tracing::{info, warn};

use crate::cache::{self, DexPoolCreatedRecord, TradeRecord};

pub struct DexEvtWebhook {
    pub redis_client: Arc<redis::Client>,
    pub http_client: Arc<reqwest::Client>,
    pub endpoint: String,
}

#[derive(Debug, Serialize)]
pub struct WebhookReq {
    pub pool_created_evts: Vec<DexPoolCreatedRecord>,
    pub trade_evts: Vec<TradeRecord>,
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

            let mut pool_created_evts = vec![];
            let mut trade_evts = vec![];

            for evt in events {
                match evt {
                    cache::DexEvent::Trade(trade_record) => trade_evts.push(trade_record),
                    cache::DexEvent::PoolCreated(dex_pool_record) => {
                        pool_created_evts.push(dex_pool_record)
                    }
                }
            }

            let req = WebhookReq {
                pool_created_evts,
                trade_evts,
            };

            info!("send {} trades to webhook: {}", events_len, self.endpoint);
            let msg = serde_json::to_string(&req)
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
