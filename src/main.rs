use std::path::PathBuf;

use anyhow::{Result, anyhow};
use clap::Parser;
use sol_dex_data_hub::{
    config::AppConfig,
    web::{self, WebAppContext},
};
use tokio::fs;
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
    web::start(context, &config.listen_on).await?;

    Ok(())
}
