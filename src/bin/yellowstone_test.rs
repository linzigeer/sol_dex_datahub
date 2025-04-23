use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use futures::{Sink, SinkExt, Stream, StreamExt, channel::mpsc};
use tokio::time::interval;
use tracing::info;
use tracing_subscriber::EnvFilter;
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcClient};
use yellowstone_grpc_proto::{
    geyser::{
        CommitmentLevel, SubscribeRequest, SubscribeRequestFilterBlocks,
        SubscribeRequestFilterBlocksMeta, SubscribeRequestFilterTransactions, SubscribeRequestPing,
        SubscribeUpdate, SubscribeUpdateBlockMeta, SubscribeUpdatePong, SubscribeUpdateSlot,
        SubscribeUpdateTransaction, subscribe_update::UpdateOneof,
    },
    tonic::{Status, codec::CompressionEncoding},
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
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_thread_names(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_ansi(false)
        .init();

    let args = Args::parse();

    let mut client = GeyserGrpcClient::build_from_shared(args.endpoint)?
        .x_token(args.x_token)?
        .accept_compressed(CompressionEncoding::Zstd)
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;
    let (mut subscribe_tx, mut stream) = client.subscribe().await?;

    let req_future = send_request(&mut subscribe_tx);
    let resp_future = process_response(&mut stream);

    futures::try_join!(req_future, resp_future)?;

    Ok(())
}

async fn send_request(
    subscribe_tx: &mut (impl Sink<SubscribeRequest, Error = mpsc::SendError> + Unpin),
) -> Result<()> {
    let programs = vec![
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P".to_owned(),
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA".to_owned(),
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_owned(),
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo".to_owned(),
        "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB".to_owned(),
    ];

    subscribe_tx
        .send(SubscribeRequest {
            // blocks_meta: maplit::hashmap! {
            //     "".to_owned() => SubscribeRequestFilterBlocksMeta {},
            // },
            blocks: maplit::hashmap! {
                "".to_owned() => SubscribeRequestFilterBlocks{
                    account_include: programs.clone(),
                    include_transactions: Some(true),
                    include_accounts: Some(false),
                    include_entries: Some(false),
                },
            },
            // transactions: maplit::hashmap! {
            //     "".to_owned() => SubscribeRequestFilterTransactions {
            //         vote: Some(false),
            //         failed: Some(false),
            //         account_include:programs.clone(),
            //         ..Default::default()
            //     }
            // },
            commitment: Some(CommitmentLevel::Confirmed as i32),
            from_slot: None,
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
}

async fn process_response(
    stream: &mut (impl Stream<Item = Result<SubscribeUpdate, Status>> + Unpin),
) -> Result<()> {
    let mut tx_cache: HashMap<u64, Vec<SubscribeUpdateTransaction>> = HashMap::new();

    while let Some(message) = stream.next().await {
        match message?.update_oneof.expect("valid message") {
            UpdateOneof::Slot(SubscribeUpdateSlot { slot, .. }) => {
                info!("slot received: {slot}");
            }
            UpdateOneof::BlockMeta(blk_meta) => {
                process_blk_meta(blk_meta, &mut tx_cache)?;
            }
            UpdateOneof::Block(blk) => {
                let txs = blk.transactions.len();
                let slot = blk.slot;
                let blk_hash = blk.blockhash;
                info!(
                    "=========================================> blk: {blk_hash}, slot: {slot}, txs: {txs}"
                );
                let tx_ids: Vec<_> = blk
                    .transactions
                    .iter()
                    .map(|it| bs58::encode(&it.signature).into_string())
                    .collect();
                info!("txids: {tx_ids:#?}");
            }
            UpdateOneof::Transaction(tx) => {
                process_tx(tx, &mut tx_cache)?;
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

fn process_tx(
    tx_resp: SubscribeUpdateTransaction,
    tx_cache: &mut HashMap<u64, Vec<SubscribeUpdateTransaction>>,
) -> Result<()> {
    let tx_info = tx_resp.transaction.as_ref();
    let tx = tx_info.and_then(|it| it.transaction.as_ref());
    let tx_meta = tx_info.and_then(|it| it.meta.as_ref());
    let tx_msg = tx.and_then(|it| it.message.as_ref());

    if tx_info.is_none() || tx.is_none() || tx_meta.is_none() || tx_msg.is_none() {
        return Ok(());
    }

    let tx_info = tx_info.unwrap();
    let tx = tx.unwrap();
    let tx_meta = tx_meta.unwrap();
    let tx_msg = tx_msg.unwrap();

    let txid = bs58::encode(&tx_info.signature).into_string();

    let mut msg_keys: Vec<_> = tx_msg
        .account_keys
        .iter()
        .map(|it| bs58::encode(it).into_string())
        .collect();

    let mut loaded_keys = vec![];
    for wk in tx_meta.loaded_writable_addresses.iter() {
        loaded_keys.push(bs58::encode(wk).into_string())
    }
    for rk in tx_meta.loaded_readonly_addresses.iter() {
        loaded_keys.push(bs58::encode(rk).into_string())
    }

    msg_keys.extend(loaded_keys.into_iter());
    let account_len = msg_keys.len();
    info!(txid, account_len);

    let logs = &tx_meta.log_messages[..];
    let ixs = &tx_msg.instructions[..];

    for (idx, ix) in ixs.iter().enumerate() {
        let prog_id = msg_keys.get(ix.program_id_index as usize).unwrap();
        let is_raydium_amm_prog = prog_id == "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
        if is_raydium_amm_prog {
            info!(?ix.data, "raydium amm program ix data ");
        }
        let swap_base_in_id = [9u8];
        let swap_base_out_id = [11u8];
        let is_swap =
            ix.data.starts_with(&swap_base_in_id) || ix.data.starts_with(&swap_base_out_id);
        if is_raydium_amm_prog && is_swap {
            let pool = ix
                .accounts
                .get(1)
                .and_then(|acc_idx| msg_keys.get(*acc_idx as usize))
                .unwrap();
            info!(pool);
        }
    }

    for innerIx in tx_meta.inner_instructions.iter() {
        // TODO: process inner instruction
    }

    let txs = tx_cache.entry(tx_resp.slot).or_default();
    txs.push(tx_resp);

    Ok(())
}

fn process_blk_meta(
    blk_meta: SubscribeUpdateBlockMeta,
    tx_cache: &mut HashMap<u64, Vec<SubscribeUpdateTransaction>>,
) -> Result<()> {
    let slot = blk_meta.slot;

    let txs = tx_cache.remove(&slot);
    if txs.is_none() {
        return Ok(());
    }

    let txs = txs.unwrap();

    for tx in txs.iter() {
        // TODO:change trnasaction timestamp
    }

    let blk_ts = blk_meta.block_time.map(|it| it.timestamp);
    let blk_height = blk_meta.block_height.map(|it| it.block_height);
    let txs = blk_meta.executed_transaction_count;
    let entries = blk_meta.entries_count;
    let ts_diff = Utc::now().timestamp() - blk_ts.unwrap_or_default();

    info!(
        slot,
        blk_ts, ts_diff, blk_height, txs, entries, "=================> block meta"
    );

    Ok(())
}
