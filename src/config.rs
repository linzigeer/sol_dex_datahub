use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub listen_on: String,
    pub ws_ticket: String,
    pub db_url: String,
    pub db_pool_max_size: u32,
    pub redis_url: String,
    pub sol_rpc_url: String,
}
