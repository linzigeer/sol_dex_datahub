use std::{collections::HashMap, time::Duration};

use clap::Parser;
use futures::{SinkExt, StreamExt};
use tokio::time::interval;
use tracing::info;
use tracing_subscriber::{EnvFilter, Registry, fmt::Layer, layer::SubscriberExt};
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterBlocks, SubscribeRequestFilterSlots,
    SubscribeRequestFilterTransactions, SubscribeRequestPing, SubscribeUpdatePong,
    SubscribeUpdateSlot, SubscribeUpdateTransaction, subscribe_update::UpdateOneof,
};

#[derive(Debug, Clone, Parser)]
#[clap(author, version, about)]
struct Args {
    /// Service endpoint
    #[clap(short, long, default_value_t = String::from("https://solana-yellowstone-grpc.publicnode.com:443"))]
    endpoint: String,

    #[clap(long)]
    x_token: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = Registry::default().with(env_filter).with(
        Layer::default()
            .with_writer(std::io::stdout)
            .with_ansi(false),
    );

    tracing::subscriber::set_global_default(subscriber)?;

    let args = Args::parse();

    let mut client = GeyserGrpcClient::build_from_shared(args.endpoint)?
        .x_token(args.x_token)?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;
    let (mut subscribe_tx, mut stream) = client.subscribe().await?;

    futures::try_join!(
        async move {
            subscribe_tx
                .send(SubscribeRequest {
                    slots: maplit::hashmap! {
                        "".to_owned() => SubscribeRequestFilterSlots {
                            filter_by_commitment: Some(true),
                            interslot_updates: Some(false)
                        }
                    },
                    blocks: maplit::hashmap! {
                        "".to_owned() => SubscribeRequestFilterBlocks {
                            account_include:vec![
                                "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_owned(),
                                // "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_owned(),
                                // "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo".to_owned()
                            ],
                            include_transactions: Some(false),
                            include_accounts: Some(false),
                            include_entries: Some(false),
                        }
                    },
                    transactions: maplit::hashmap! {
                        "".to_owned() => SubscribeRequestFilterTransactions {
                            vote: Some(false),
                            failed: Some(false),
                            account_include:vec![
                                "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_owned(),
                                // "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_owned(),
                                // "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo".to_owned()
                            ],
                            ..Default::default()
                        }
                    },
                    commitment: Some(CommitmentLevel::Confirmed as i32),
                    ..Default::default()
                })
                .await?;

            let mut timer = interval(Duration::from_secs(3));
            let mut id = 0;
            loop {
                timer.tick().await;
                id += 1;
                subscribe_tx
                    .send(SubscribeRequest {
                        ping: Some(SubscribeRequestPing { id }),
                        ..Default::default()
                    })
                    .await?;
            }
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        },
        async move {
            let mut blk_txs: HashMap<u64, Vec<SubscribeUpdateTransaction>> = HashMap::new();
            while let Some(message) = stream.next().await {
                match message?.update_oneof.expect("valid message") {
                    UpdateOneof::Slot(SubscribeUpdateSlot { slot, .. }) => {
                        info!("slot received: {slot}");
                    }
                    UpdateOneof::Block(blk) => {
                        let blk_ts = blk.block_time.unwrap().timestamp;
                        info!(
                            "block slot: {}, blk_ts: {}, txs: {}",
                            blk.slot,
                            blk_ts,
                            blk.transactions.len()
                        );
                        let txs = blk_txs.remove(&blk.slot).unwrap();
                        for tx in txs {
                            // let meta = tx.meta.unwrap();
                            // let transaction = tx.transaction.unwrap();
                            // let txid = bs58::encode(&transaction.signatures[0]).into_string();
                            let transaction = tx.transaction.unwrap();
                            let failed = transaction.meta.unwrap().err.is_some();
                            let txid = bs58::encode(transaction.signature).into_string();
                            info!(
                                "slot: {}, blk_ts: {}, tx: {txid}, failed: {failed}",
                                blk.slot, blk_ts
                            );
                        }
                    }
                    UpdateOneof::Transaction(tx) => {
                        let txs = blk_txs.entry(tx.slot).or_default();
                        txs.push(tx);
                        // blk_txs.insert(tx.slot, tx);
                        // let txid = bs58::encode(tx.transaction.unwrap().signature).into_string();
                        // let ts = blk_ts.get(&tx.slot).copied();
                        // info!(
                        //     "transaction at slot: {}, blk_ts: {ts:?}, txid: {txid}",
                        //     tx.slot
                        // );
                    }
                    UpdateOneof::Ping(_msg) => {
                        info!("ping received");
                    }
                    UpdateOneof::Pong(SubscribeUpdatePong { id }) => {
                        info!("pong received: id#{id}");
                    }
                    msg => anyhow::bail!("received unexpected message: {msg:?}"),
                }
            }
            Ok::<(), anyhow::Error>(())
        }
    )?;

    Ok(())
}
