use std::{sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use axum::extract::ws::Utf8Bytes;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use tokio::sync::{RwLock, broadcast};

use crate::config::AppConfig;

#[derive(Clone)]
pub struct WebAppContext {
    pub ws_broadcast: broadcast::Sender<Utf8Bytes>,
    pub ws_connected: Arc<RwLock<bool>>,
    pub db_pool: MySqlPool,
    pub redis_client: Arc<redis::Client>,
    pub sol_rpc_client: Arc<RpcClient>,
}

impl WebAppContext {
    pub async fn init(config: &AppConfig) -> Result<Self> {
        let db_pool = MySqlPoolOptions::new()
            .min_connections(5)
            .max_connections(config.db_pool_max_size)
            .acquire_timeout(Duration::from_secs(3))
            .connect(&config.db_url)
            .await
            .map_err(|err| anyhow!("connect to database {} error: {}", config.db_url, err))?;

        let sol_rpc_client = RpcClient::new_with_timeout_and_commitment(
            config.sol_rpc_url.clone(),
            Duration::from_secs(5),
            CommitmentConfig::processed(),
        );
        let sol_rpc_client = Arc::new(sol_rpc_client);

        let redis_client = redis::Client::open(config.redis_url.as_str())?;
        let redis_client = Arc::new(redis_client);
        let (tx, _) = broadcast::channel(100);

        Ok(Self {
            ws_connected: Arc::new(RwLock::new(false)),
            ws_broadcast: tx,
            db_pool,
            redis_client,
            sol_rpc_client,
        })
    }
}
