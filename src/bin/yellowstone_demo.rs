use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use clap::Parser;
use futures::{SinkExt, StreamExt};
use solana_sdk::pubkey::Pubkey;
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

    let programs = vec![
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_owned(),
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_owned(),
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA".to_owned(),
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo".to_owned(),
        "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB".to_owned(),
    ];

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
                            account_include:programs.clone(),
                            include_transactions: Some(false),
                            include_accounts: Some(false),
                            include_entries: Some(false),
                        }
                    },
                    transactions: maplit::hashmap! {
                        "".to_owned() => SubscribeRequestFilterTransactions {
                            vote: Some(false),
                            failed: Some(false),
                            account_include:programs.clone(),
                            ..Default::default()
                        }
                    },
                    commitment: Some(CommitmentLevel::Processed as i32),
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
                        if !blk_txs.contains_key(&blk.slot) {
                            continue;
                        }
                        let txs = blk_txs.remove(&blk.slot).unwrap();
                        let ts_diff = Utc::now().timestamp() - blk_ts;
                        for tx in txs {
                            let tx_info = tx.transaction.unwrap();
                            let tx_meta = tx_info.meta.unwrap();
                            let failed = tx_meta.err.is_some();
                            let txid = bs58::encode(tx_info.signature).into_string();
                            info!("=======> tx: {txid}");

                            let tx = tx_info.transaction.unwrap();
                            let tx_msg = tx.message.unwrap();
                            let mut msg_keys = tx_msg.account_keys.clone();

                            let mut loaded_keys = vec![];
                            for wk in tx_meta.loaded_writable_addresses.iter() {
                                loaded_keys.push(wk.clone())
                            }
                            for rk in tx_meta.loaded_readonly_addresses.iter() {
                                loaded_keys.push(rk.clone())
                            }

                            msg_keys.extend(loaded_keys.into_iter());
                            info!("//////////////  all account len: {}", msg_keys.len());
                            for (idx, ix) in tx_msg.instructions.iter().enumerate() {
                                let ix = ix.clone();
                                let ddd: Vec<_> = msg_keys
                                    .iter()
                                    .map(|it| bs58::encode(it).into_string())
                                    .collect();
                                let prog_id = ddd.get(ix.program_id_index as usize).unwrap();
                                if prog_id == "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" {
                                    info!("all keys: {ddd:#?}");
                                    info!("program: {idx}, {prog_id}");
                                    info!("accounts: {:?}", ix.accounts);
                                    for account_idx in ix.accounts.iter() {
                                        info!("get accounts: {account_idx} .....");
                                        let account_key =
                                            msg_keys.get(*account_idx as usize).unwrap();
                                        let account_key = bs58::encode(&account_key).into_string();
                                        info!("account {}: {}", account_idx, account_key);
                                    }

                                    if let Some(iix) = tx_meta
                                        .inner_instructions
                                        .iter()
                                        .find(|it| it.index as usize == idx)
                                    {
                                        for x in iix.instructions.iter() {
                                            let pid = ddd.get(x.program_id_index as usize).unwrap();
                                            info!("ix idx: {idx} program: {pid}");
                                        }
                                    }
                                }
                            }
                            for iixs in tx_meta.inner_instructions.iter() {
                                let iixs = iixs.clone();
                                let ddd: Vec<_> = msg_keys
                                    .iter()
                                    .map(|it| bs58::encode(it).into_string())
                                    .collect();
                                for (idx, iix) in iixs.instructions.iter().enumerate() {
                                    let prog_id = ddd.get(iix.program_id_index as usize).unwrap();
                                    if prog_id == "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" {
                                        info!("all keys: {ddd:#?}");
                                        info!("program: {idx}, {prog_id}");
                                        info!("accounts: {:?}", iix.accounts);
                                        for account_idx in iix.accounts.iter() {
                                            info!("get accounts: {account_idx} .....");
                                            let account_key =
                                                msg_keys.get(*account_idx as usize).unwrap();
                                            let account_key: Pubkey =
                                                borsh::from_slice(account_key)?;
                                            // let account_key =
                                            //     bs58::encode(&account_key).into_string();
                                            info!("account {}: {}", account_idx, account_key);
                                        }

                                        let ix_next1 =
                                            iixs.instructions.get(idx + 1).cloned().unwrap();
                                        let pid_next1 =
                                            ddd.get(ix_next1.program_id_index as usize).unwrap();
                                        info!("ix next1 program: {pid_next1}");

                                        let ix_next2 =
                                            iixs.instructions.get(idx + 2).cloned().unwrap();
                                        let pid_next2 =
                                            ddd.get(ix_next2.program_id_index as usize).unwrap();
                                        info!("ix next2 program: {pid_next2}");
                                    }
                                }
                            }
                            info!(
                                "slot: {}, blk_ts: {}, ts diff: {ts_diff} tx: {txid}, failed: {failed}",
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
