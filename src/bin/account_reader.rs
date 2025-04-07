use anyhow::Result;
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

    meteora_dlmm::read_dlmm_position_tokens(&rpc_client).await?;

    Ok(())
}

mod meteora_dlmm {

    use std::collections::HashSet;

    use super::*;
    use borsh::BorshDeserialize;
    use itertools::Itertools;
    use num_bigint::{BigInt, BigUint};

    #[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
    pub struct StaticParameters {
        pub base_factor: u16,
        pub filter_period: u16,
        pub decay_period: u16,
        pub reduction_factor: u16,
        pub variable_fee_control: u32,
        pub max_volatility_accumulator: u32,
        pub min_bin_id: i32,
        pub max_bin_id: i32,
        pub protocol_share: u16,
        pub base_fee_power_factor: u8,
        pub padding: [u8; 5],
    }

    #[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
    pub struct VariableParameters {
        pub volatility_accumulator: u32,
        pub volatility_reference: u32,
        pub index_reference: i32,
        pub padding: [u8; 4],
        pub last_update_timestamp: i64,
        pub padding1: [u8; 8],
    }

    #[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
    pub struct ProtocolFee {
        pub amount_x: u64,
        pub amount_y: u64,
    }

    #[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
    pub struct RewardInfo {
        pub mint: Pubkey,
        pub vault: Pubkey,
        pub funder: Pubkey,
        pub reward_duration: u64,
        pub reward_duration_end: u64,
        pub reward_rate: u128,
        pub last_update_time: u64,
        pub cumulative_seconds_with_empty_liquidity_reward: u64,
    }

    #[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
    pub struct LbPair {
        pub parameters: StaticParameters,
        pub v_parameters: VariableParameters,
        pub bump_seed: [u8; 1],
        pub bin_step_seed: [u8; 2],
        pub pair_type: u8,
        pub active_id: i32,
        pub bin_step: u16,
        pub status: u8,
        pub require_base_factor_seed: u8,
        pub base_factor_seed: [u8; 2],
        pub activation_type: u8,
        pub creator_pool_on_off_control: u8,
        pub token_x_mint: Pubkey,
        pub token_y_mint: Pubkey,
        pub reserve_x: Pubkey,
        pub reserve_y: Pubkey,
        pub protocol_fee: ProtocolFee,
        pub padding1: [u8; 32],
        pub reward_infos: [RewardInfo; 2],
        pub oracle: Pubkey,
        pub bin_array_bitmap: [u64; 16],
        pub last_updated_at: i64,
        pub padding2: [u8; 32],
        pub pre_activation_swap_address: Pubkey,
        pub base_key: Pubkey,
        pub activation_point: u64,
        pub pre_activation_duration: u64,
        pub padding3: [u8; 8],
        pub padding4: u64,
        pub creator: Pubkey,
        pub token_mint_x_program_flag: u8,
        pub token_mint_y_program_flag: u8,
        pub reserved: [u8; 22],
    }

    #[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
    pub struct UserRewardInfo {
        pub reward_per_token_completes: [u128; 2],
        pub reward_pendings: [u64; 2],
    }
    #[derive(Clone, Debug, BorshDeserialize, PartialEq, Copy)]
    pub struct FeeInfo {
        pub fee_x_per_token_complete: u128,
        pub fee_y_per_token_complete: u128,
        pub fee_x_pending: u64,
        pub fee_y_pending: u64,
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

    const MAX_BIN_PER_ARRAY: i64 = 70;
    pub async fn read_dlmm_position_tokens(rpc_client: &RpcClient) -> Result<()> {
        let lb_pair = pubkey!("6WTbcDmtqDNwxxLe9YzHzpSSBKQ7AduZG7SmYWpRwjZZ");
        let position = pubkey!("8Zhc5rkShtuzc5YKQZiKCfFdr2cicSNxuyDqm6vwK4dN");
        // let position = pubkey!("GMEgkBPgasHcopyWhrPg5AB3fwYt4zAmvieErftPGQKy");

        let lb_pair_bytes = rpc_client.get_account_data(&lb_pair).await?;
        let lb_pair_data: LbPair = borsh1::try_from_slice_unchecked(&lb_pair_bytes[8..])?;
        info!(
            "{lb_pair} active id is: {}, x_mint: {}, y_mint: {}",
            lb_pair_data.active_id, lb_pair_data.token_x_mint, lb_pair_data.token_y_mint
        );
        let _base_decimals = 6;
        let _quote_decimals = 9;

        let position_bytes = rpc_client.get_account_data(&position).await?;
        let position_data: PositionV2 = borsh1::try_from_slice_unchecked(&position_bytes[8..])?;
        info!(
            "{position} bin range is: ({}, {}), liquidity_shares: {}",
            position_data.lower_bin_id,
            position_data.upper_bin_id,
            position_data.liquidity_shares.len()
        );

        let position_bin_array_keys: Vec<_> = (position_data.lower_bin_id
            ..=position_data.upper_bin_id)
            .map(|id| {
                let idx = bin_id_to_bin_array_idx(id as i64);
                derive_bin_array(lb_pair, idx)
            })
            .unique()
            .collect();
        println!("bin_array accounts: {position_bin_array_keys:#?}");

        let position_bin_array_accounts = rpc_client
            .get_multiple_accounts(&position_bin_array_keys)
            .await?;

        let mut position_bin_arrays = vec![];
        for ba_acc in position_bin_array_accounts {
            let ba: BinArray = borsh1::try_from_slice_unchecked(&ba_acc.unwrap().data)?;
            position_bin_arrays.push(ba);
        }

        let mut amount_x = 0u64;
        let mut amount_y = 0u64;

        for pba in position_bin_arrays {
            for (idx, bin) in pba.bins.iter().enumerate() {
                let pos_share = position_data.liquidity_shares[idx];

                let x_amount = BigUint::from(pos_share) * BigUint::from(bin.amount_x)
                    / BigUint::from(bin.liquidity_supply);
                let y_amount = BigUint::from(pos_share) * BigUint::from(bin.amount_y)
                    / BigUint::from(bin.liquidity_supply);

                amount_x += u64::try_from(x_amount)?;
                amount_y += u64::try_from(y_amount)?;
            }
        }

        println!("amount_x: {amount_x}, amount_y: {amount_y}");

        Ok(())
    }

    pub fn derive_bin_array(lb_pair: Pubkey, idx: i64) -> Pubkey {
        let dlmm_prog = pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");
        let bin_array_idx_bytes = idx.to_le_bytes();

        let (derive_pda, _) = Pubkey::find_program_address(
            &[b"bin_array", &lb_pair.to_bytes(), &bin_array_idx_bytes],
            &dlmm_prog,
        );
        derive_pda
    }

    pub fn bin_id_to_bin_array_idx(bin_id: i64) -> i64 {
        let idx = bin_id / MAX_BIN_PER_ARRAY;
        let mod_val = bin_id % MAX_BIN_PER_ARRAY;
        if bin_id.is_negative() && mod_val != 0 {
            idx - 1
        } else {
            idx
        }
    }
}

mod stream_flow {
    use super::*;
    use borsh::BorshDeserialize;

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

    pub async fn read_fees(rpc_client: &RpcClient) -> Result<()> {
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
}
