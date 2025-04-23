use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use borsh::BorshDeserialize;
use futures::StreamExt;
use itertools::Itertools;
use num_bigint::BigUint;
use once_cell::sync::Lazy;
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_pubsub_client::nonblocking::pubsub_client::PubsubClient;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{
    config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    filter::RpcFilterType,
};
use solana_sdk::pubkey;
use solana_sdk::{borsh1, commitment_config::CommitmentConfig, pubkey::Pubkey};

#[derive(Debug, BorshDeserialize, Clone)]
pub struct Dish {
    pub creator: Pubkey,
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub pool: Pubkey,
    pub position: Pubkey,
    pub insurance_info: InsuranceInfo,
    pub locked_token_info: LockedTokenInfo,
    pub is_granduated: bool,
    pub withdrawed_token_vault_amt: u64,
    pub withdrawed_wsol_vault_amt: u64,
}

#[derive(Debug, BorshDeserialize, Clone, Copy)]
pub struct InsuranceInfo {
    pub insurance_lp: Pubkey,
    pub insurance_amt: u64,
    pub insurance_price: u64,
    pub insurance_state: InsuranceState,
    pub deposit_ts: i64,
    pub depoly_ts: i64,
    pub refund_ts: i64,
}

#[derive(Debug, BorshDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum InsuranceState {
    None,
    Deposited,
    Deployed,
    Refunded,
}

#[derive(Debug, BorshDeserialize, Clone, Copy)]
pub struct LockedTokenInfo {
    pub mint: Pubkey,
    pub locked_amount: u64,
    pub lock_status: LockState,
    pub lock_ts: i64,
    pub unlock_ts: u64,
    pub claim_ts: i64,
}

#[derive(BorshDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LockState {
    None,
    Locked,
    Unlocked,
}

#[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
pub struct PositionV2 {
    pub lb_pair: Pubkey,
    pub owner: Pubkey,
    pub liquidity_shares: [u128; 70],
    pub reward_infos: [UserRewardInfo; 70],
    pub fee_infos: [FeeInfo; 70],
    pub lower_bin_id: i32,
    pub upper_bin_id: i32,
    pub last_updated_at: i64,
    pub total_claimed_fee_x_amount: u64,
    pub total_claimed_fee_y_amount: u64,
    pub total_claimed_rewards: [u64; 2],
    pub operator: Pubkey,
    pub lock_release_point: u64,
    pub padding0: u8,
    pub fee_owner: Pubkey,
    pub reserved: [u8; 87],
}

#[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
pub struct FeeInfo {
    pub fee_x_per_token_complete: u128,
    pub fee_y_per_token_complete: u128,
    pub fee_x_pending: u64,
    pub fee_y_pending: u64,
}

#[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
pub struct UserRewardInfo {
    pub reward_per_token_completes: [u128; 2],
    pub reward_pendings: [u64; 2],
}

#[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
pub struct Bin {
    pub amount_x: u64,
    pub amount_y: u64,
    pub price: u128,
    pub liquidity_supply: u128,
    pub reward_per_token_stored: [u128; 2],
    pub fee_amount_x_per_token_stored: u128,
    pub fee_amount_y_per_token_stored: u128,
    pub amount_x_in: u128,
    pub amount_y_in: u128,
}

#[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
pub struct BinArray {
    pub index: i64,
    pub version: u8,
    pub padding: [u8; 7],
    pub lb_pair: Pubkey,
    pub bins: [Bin; 70],
}

const DLMM_PROG: Pubkey = pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");
const MAX_BIN_PER_ARRAY: i32 = 70;
const COOKING_PROGRAM_ID: Pubkey = pubkey!("C6693Z6TGS4WrDQDXWCUKFhCYbKv2NarV43soRNyfuoA");
const RPC_URL: &str = "https://devnet.helius-rpc.com/?api-key=6dc55e66-39de-43dd-a297-0c79fda11cf2";
const PUBSUB_URL: &str =
    "wss://devnet.helius-rpc.com/?api-key=6dc55e66-39de-43dd-a297-0c79fda11cf2";

static RPC_ACCOUNT_INFO_CONFIG: Lazy<RpcAccountInfoConfig> = Lazy::new(|| RpcAccountInfoConfig {
    encoding: Some(UiAccountEncoding::Base64),
    data_slice: None,
    commitment: None,
    min_context_slot: None,
});

static RPC_PROGRAM_ACCOUNTS_CONFIG: Lazy<RpcProgramAccountsConfig> =
    Lazy::new(|| RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::DataSize(363)]),
        account_config: RPC_ACCOUNT_INFO_CONFIG.clone(),
        with_context: Some(true),
        sort_results: None,
    });

#[tokio::main()]
pub async fn main() -> Result<()> {
    let rpc_client =
        RpcClient::new_with_commitment(RPC_URL.to_string(), CommitmentConfig::confirmed());

    let mut latest_slot = 375224796;

    loop {
        let dishes = fill_missed_dishes(&rpc_client, latest_slot).await?;

        let mut curve_position_keys = vec![];
        let mut insurance_position_keys = vec![];
        for (key, dish) in dishes {
            if !dish.is_granduated {
                curve_position_keys.push(dish.position);
            } else if dish.insurance_info.depoly_ts > 0
                && dish.insurance_info.insurance_state == InsuranceState::Deployed
            {
                insurance_position_keys.push(dish.insurance_info.insurance_lp);
            }
            println!("dish account {key}: {dish:#?} \n");
        }

        let curve_positions = batch_get_curve_positions(&rpc_client, &curve_position_keys).await?;
        for (pos_key, curve_pos) in curve_positions {
            let pos_total_y_amt = get_positions_total_amount_y(&rpc_client, &curve_pos).await?;
            let lb_pair = curve_pos.lb_pair;
            let lower_bin_id = curve_pos.lower_bin_id;
            let upper_bin_id = curve_pos.upper_bin_id;

            println!(
                "=== curve lb_pair: {}, position: {}, min_bin_id: {}, max_bin_id: {},  total sol: {}",
                lb_pair, pos_key, lower_bin_id, upper_bin_id, pos_total_y_amt,
            );
        }

        let insurance_positions =
            batch_get_curve_positions(&rpc_client, &insurance_position_keys).await?;
        for (pos_key, pos) in insurance_positions {
            let pos_total_y_amt = get_positions_total_amount_y(&rpc_client, &pos).await?;
            let lb_pair = pos.lb_pair;
            let lower_bin_id = pos.lower_bin_id;
            let upper_bin_id = pos.upper_bin_id;

            println!(
                "*** insurance lb_pair: {}, position: {}, min_bin_id: {}, max_bin_id: {},  total sol: {}",
                lb_pair, pos_key, lower_bin_id, upper_bin_id, pos_total_y_amt,
            );
        }

        println!("connecting to pubsub url ......");
        let pubsub_client = PubsubClient::new(PUBSUB_URL).await?;

        let (mut resp_stream, _) = pubsub_client
            .program_subscribe(
                &COOKING_PROGRAM_ID,
                Some(RPC_PROGRAM_ACCOUNTS_CONFIG.clone()),
            )
            .await?;

        while let Some(resp) = resp_stream.next().await {
            latest_slot = resp.context.slot;

            let key = resp.value.pubkey;
            let acc_data = resp.value.account.data.decode().unwrap_or_default();
            let dish: Dish = borsh1::try_from_slice_unchecked(&acc_data[8..])?;
            println!("dish account {key} updated: {dish:#?} \n");
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    #[allow(unreachable_code)]
    Ok(())
}

pub async fn get_positions_total_amount_y(
    rpc_client: &RpcClient,
    position: &PositionV2,
) -> Result<u64> {
    let lb_pair = position.lb_pair;
    let lower_bin_id = position.lower_bin_id;
    let upper_bin_id = position.upper_bin_id;
    let pos_bin_arrays = batch_get_bin_arrays(rpc_client, lb_pair, lower_bin_id).await?;
    let mut pos_total_y_amt = 0;
    let mut pos_share_idx = 0;
    for (_, bin_array) in pos_bin_arrays {
        let bin_array_lower_bin_id = bin_array.index as i32 * MAX_BIN_PER_ARRAY;
        for (idx, bin) in bin_array.bins.iter().enumerate() {
            let bin_id = bin_array_lower_bin_id + idx as i32;
            if bin_id >= lower_bin_id && bin_id <= upper_bin_id && bin.liquidity_supply > 0 {
                let liq_share = position.liquidity_shares[pos_share_idx];
                pos_share_idx += 1;
                let amount_y_in_bin = BigUint::from(bin.amount_y) * BigUint::from(liq_share)
                    / BigUint::from(bin.liquidity_supply);

                pos_total_y_amt += u64::try_from(amount_y_in_bin)?;
            }
        }
    }

    Ok(pos_total_y_amt)
}

pub async fn read_position_from_chain(
    rpc_client: &RpcClient,
    pos_key: &Pubkey,
) -> Result<Option<PositionV2>> {
    let resp = rpc_client
        .get_account_with_commitment(pos_key, CommitmentConfig::processed())
        .await?;
    if let Some(acc) = resp.value {
        let pos: PositionV2 = borsh1::try_from_slice_unchecked(&acc.data[8..])?;
        return Ok(Some(pos));
    }

    Ok(None)
}

pub async fn batch_get_bin_arrays(
    rpc_client: &RpcClient,
    lb_pair: Pubkey,
    lower_bin_id: i32,
) -> Result<HashMap<Pubkey, BinArray>> {
    let lower_bin_array_idx = bin_id_to_bin_array_idx(lower_bin_id);
    let upper_bin_array_idx = lower_bin_array_idx + 1;
    let position_bin_array_keys: Vec<_> = [lower_bin_array_idx, upper_bin_array_idx]
        .into_iter()
        .map(|bin_array_idx| derive_bin_array(lb_pair, bin_array_idx as i64))
        .unique()
        .collect();

    let position_bin_array_accounts = rpc_client
        .get_multiple_accounts(&position_bin_array_keys)
        .await?;

    let mut position_bin_arrays_map = HashMap::new();
    for (idx, ba_acc) in position_bin_array_accounts.into_iter().enumerate() {
        let bin_array_pubkey = position_bin_array_keys[idx];
        let ba: BinArray = borsh1::try_from_slice_unchecked(&ba_acc.unwrap().data[8..])?;
        position_bin_arrays_map.insert(bin_array_pubkey, ba);
    }

    Ok(position_bin_arrays_map)
}

pub async fn batch_get_curve_positions(
    rpc_client: &RpcClient,
    pos_keys: &[Pubkey],
) -> Result<HashMap<Pubkey, PositionV2>> {
    if pos_keys.len() > 100 {
        anyhow::bail!("must less than 100 positions");
    }

    if pos_keys.is_empty() {
        return Ok(HashMap::new());
    }

    let mut positions: HashMap<Pubkey, PositionV2> = HashMap::new();

    let resp = rpc_client
        .get_multiple_accounts_with_commitment(pos_keys, CommitmentConfig::processed())
        .await?;

    for (key, maybe_acc) in pos_keys.iter().zip(resp.value.iter()) {
        if let Some(acc) = maybe_acc {
            let pos: PositionV2 = borsh1::try_from_slice_unchecked(&acc.data[8..])?;
            positions.insert(*key, pos);
        }
    }

    Ok(positions)
}

pub async fn fill_missed_dishes(
    rpc_client: &RpcClient,
    min_slot: u64,
) -> Result<HashMap<Pubkey, Dish>> {
    let account_info_config = RpcAccountInfoConfig {
        min_context_slot: Some(min_slot),
        ..RPC_ACCOUNT_INFO_CONFIG.clone()
    };

    let rpc_program_accounts_config = RpcProgramAccountsConfig {
        account_config: account_info_config,
        ..RPC_PROGRAM_ACCOUNTS_CONFIG.clone()
    };

    println!("rpc program accounts config: {rpc_program_accounts_config:#?}");

    let accounts = rpc_client
        .get_program_accounts_with_config(&COOKING_PROGRAM_ID, rpc_program_accounts_config)
        .await
        .map_err(|err| anyhow::anyhow!("get accounts error: {err}"))?;

    let mut dishes: HashMap<Pubkey, Dish> = HashMap::new();

    for acc in accounts {
        let key = acc.0;
        let acc_data = acc.1.data;
        let dish: Dish = borsh1::try_from_slice_unchecked(&acc_data[8..])
            .map_err(|err| anyhow::anyhow!("parse dish error: {err}"))?;
        dishes.insert(key, dish);
    }

    Ok(dishes)
}

pub fn derive_bin_array(lb_pair: Pubkey, bin_array_idx: i64) -> Pubkey {
    let bin_array_idx_bytes = bin_array_idx.to_le_bytes();

    let (derive_pda, _) = Pubkey::find_program_address(
        &[b"bin_array", &lb_pair.to_bytes(), &bin_array_idx_bytes],
        &DLMM_PROG,
    );
    derive_pda
}

pub fn bin_id_to_bin_array_idx(bin_id: i32) -> i32 {
    let idx = bin_id / MAX_BIN_PER_ARRAY;
    let mod_val = bin_id % MAX_BIN_PER_ARRAY;
    if bin_id.is_negative() && mod_val != 0 {
        idx - 1
    } else {
        idx
    }
}
