use std::{sync::Arc, time::Duration};

use anyhow::Result;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct WebAppContext {
    pub redis_client: Arc<redis::Client>,
    pub sol_rpc_client: Arc<RpcClient>,
}

impl WebAppContext {
    pub async fn init(config: &AppConfig) -> Result<Self> {
        let sol_rpc_client = RpcClient::new_with_timeout_and_commitment(
            config.sol_rpc_url.clone(),
            Duration::from_secs(5),
            CommitmentConfig::processed(),
        );
        let sol_rpc_client = Arc::new(sol_rpc_client);

        let redis_client = redis::Client::open(config.redis_url.as_str())?;
        let redis_client = Arc::new(redis_client);

        Ok(Self {
            redis_client,
            sol_rpc_client,
        })
    }
}
