use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub listen_on: String,
    pub webhook_enpoint: String,
    pub redis_url: String,
    pub sol_rpc_url: String,
}
