use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use clap::Parser;
use sol_dex_data_hub::{
    config::AppConfig,
    qn_req_processor,
    web::{self, WebAppContext},
    webhook::DexEvtWebhook,
};
use tokio::fs;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, Registry, fmt::Layer, layer::SubscriberExt};

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[arg(long, short)]
    pub config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = Registry::default().with(env_filter).with(
        Layer::default()
            .with_writer(std::io::stdout)
            .with_ansi(false),
    );

    tracing::subscriber::set_global_default(subscriber)?;

    let cli = Cli::parse();
    let content = fs::read_to_string(cli.config).await?;
    let config = serde_json::from_str::<AppConfig>(&content)
        .map_err(|err| anyhow!("parse config json file error: {err}"))?;

    let context = WebAppContext::init(&config).await?;

    let redis_client = context.redis_client.clone();
    // process quick node stream
    tokio::spawn(async move {
        loop {
            let redis_client = redis_client.clone();
            match qn_req_processor::start(redis_client).await {
                Ok(_) => info!("qn request processor successed"),
                Err(err) => error!("qn reqwest processor error: {err}"),
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    let redis_client = context.redis_client.clone();
    let webhook_endpoint = config.webhook_enpoint.clone();
    let http_client = Arc::new(
        reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_millis(200))
            .timeout(Duration::from_secs(1))
            .build()?,
    );
    tokio::spawn(async move {
        loop {
            let redis_client = redis_client.clone();
            let webhook = DexEvtWebhook {
                redis_client,
                http_client: http_client.clone(),
                endpoint: webhook_endpoint.clone(),
            };
            match webhook.start().await {
                Ok(_) => info!("webhook processor successed"),
                Err(err) => error!("webhook processor error: {err}"),
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });

    web::start(context, &config.listen_on).await?;

    Ok(())
}
