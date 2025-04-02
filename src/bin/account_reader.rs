use anyhow::Result;
use borsh::BorshDeserialize;
use sol_dex_data_hub::meteora::damm::accounts::MeteoraDammPool;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{borsh1, pubkey};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use tracing::info;
use tracing_subscriber::{EnvFilter, Registry, fmt::Layer, layer::SubscriberExt};

#[tokio::main]
async fn main() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = Registry::default().with(env_filter).with(
        Layer::default()
            .with_writer(std::io::stdout)
            .with_ansi(false),
    );

    tracing::subscriber::set_global_default(subscriber)?;

    let rpc_client = RpcClient::new_with_commitment(
        "https://omniscient-snowy-sunset.solana-mainnet.quiknode.pro/0a052cfd0f79310032149e1a170e49617f4821b0/".to_string(), CommitmentConfig::confirmed());

    let b = rpc_client
        .get_account_data(&pubkey!("B743wFVk2pCYhV91cn287e1xY7f1vt4gdY48hhNiuQmT"))
        .await?;

    let v: Fees = borsh1::try_from_slice_unchecked(&b)?;
    let size = v.v.len();

    for x in v.v {
        info!(
            "fees: {}, pf: {}, strm: {}",
            x.key, x.partnet_fee, x.strm_fee
        );
    }

    info!("total {} fees", size);

    let meteora_damm_pool = pubkey!("HrW9pAMg7kLyt9kpp5N77xBcZJQXdrdP97Qtd2XvZUQB");
    let b = rpc_client.get_account_data(&meteora_damm_pool).await?;
    let f = b.len();
    // let pool: MeteoraDammPool = borsh1::try_from_slice_unchecked(&b)?;
    let pool = MeteoraDammPool::deserialize(&mut b.as_ref())?;
    info!("pool: {pool:#?}");
    info!("{}, {}", f, size_of_val(&pool));

    Ok(())
}

#[derive(Debug, BorshDeserialize)]
pub struct Item {
    pub key: Pubkey,
    pub partnet_fee: f32,
    pub strm_fee: f32,
}

#[derive(Debug, BorshDeserialize)]
pub struct Fees {
    pub v: Vec<Item>,
}
