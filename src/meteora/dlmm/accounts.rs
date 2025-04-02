use std::str::FromStr;

use anyhow::Result;
use borsh::BorshDeserialize;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct StaticParameters {
    /// Used for base fee calculation. base_fee_rate = base_factor * bin_step
    pub base_factor: u16,
    /// Filter period determine high frequency trading time window.
    pub filter_period: u16,
    /// Decay period determine when the volatile fee start decay / decrease.
    pub decay_period: u16,
    /// Reduction factor controls the volatile fee rate decrement rate.
    pub reduction_factor: u16,
    /// Used to scale the variable fee component depending on the dynamic of the market
    pub variable_fee_control: u32,
    /// Maximum number of bin crossed can be accumulated. Used to cap volatile fee rate.
    pub max_volatility_accumulator: u32,
    /// Min bin id supported by the pool based on the configured bin step.
    pub min_bin_id: i32,
    /// Max bin id supported by the pool based on the configured bin step.
    pub max_bin_id: i32,
    /// Portion of swap fees retained by the protocol by controlling protocol_share parameter. protocol_swap_fee = protocol_share * total_swap_fee
    pub protocol_share: u16,
    /// Padding for bytemuck safe alignment
    pub _padding: [u8; 6],
}

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct VariableParameters {
    /// Volatility accumulator measure the number of bin crossed since reference bin ID. Normally (without filter period taken into consideration), reference bin ID is the active bin of last swap.
    /// It affects the variable fee rate
    pub volatility_accumulator: u32,
    /// Volatility reference is decayed volatility accumulator. It is always <= volatility_accumulator
    pub volatility_reference: u32,
    /// Active bin id of last swap.
    pub index_reference: i32,
    /// Padding for bytemuck safe alignment
    pub _padding: [u8; 4],
    /// Last timestamp the variable parameters was updated
    pub last_update_timestamp: i64,
    /// Padding for bytemuck safe alignment
    pub _padding_1: [u8; 8],
}

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct ProtocolFee {
    pub amount_x: u64,
    pub amount_y: u64,
}

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct RewardInfo {
    /// Reward token mint.
    pub mint: Pubkey,
    /// Reward vault token account.
    pub vault: Pubkey,
    /// Authority account that allows to fund rewards
    pub funder: Pubkey,
    /// TODO check whether we need to store it in pool
    pub reward_duration: u64, // 8
    /// TODO check whether we need to store it in pool
    pub reward_duration_end: u64, // 8
    /// TODO check whether we need to store it in pool
    pub reward_rate: u128, // 8
    /// The last time reward states were updated.
    pub last_update_time: u64, // 8
    /// Accumulated seconds where when farm distribute rewards, but the bin is empty. The reward will be accumulated for next reward time window.
    pub cumulative_seconds_with_empty_liquidity_reward: u64,
}

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct LbPair {
    pub discriminator: u64,
    pub parameters: StaticParameters,
    pub v_parameters: VariableParameters,
    pub bump_seed: [u8; 1],
    /// Bin step signer seed
    pub bin_step_seed: [u8; 2],
    /// Type of the pair
    pub pair_type: u8,
    /// Active bin id
    pub active_id: i32,
    /// Bin step. Represent the price increment / decrement.
    pub bin_step: u16,
    /// Status of the pair. Check PairStatus enum.
    pub status: u8,
    /// Require base factor seed
    pub require_base_factor_seed: u8,
    /// Base factor seed
    pub base_factor_seed: [u8; 2],
    /// Activation type
    pub activation_type: u8,
    /// Allow pool creator to enable/disable pool with restricted validation. Only applicable for customizable permissionless pair type.
    pub creator_pool_on_off_control: u8,
    /// Token X mint
    pub token_x_mint: Pubkey,
    /// Token Y mint
    pub token_y_mint: Pubkey,
    /// LB token X vault
    pub reserve_x: Pubkey,
    /// LB token Y vault
    pub reserve_y: Pubkey,
    /// Uncollected protocol fee
    pub protocol_fee: ProtocolFee,
    /// _padding_1, previous Fee owner, BE CAREFUL FOR TOMBSTONE WHEN REUSE !!
    pub _padding_1: [u8; 32],
    /// Farming reward information
    pub reward_infos: [RewardInfo; 2], // TODO: Bug in anchor IDL parser when using InitSpace macro. Temp hardcode it. https://github.com/coral-xyz/anchor/issues/2556
    /// Oracle pubkey
    pub oracle: Pubkey,
    /// Packed initialized bin array state
    pub bin_array_bitmap: [u64; 16], // store default bin id from -512 to 511 (bin id from -35840 to 35840, price from 2.7e-16 to 3.6e15)
    /// Last time the pool fee parameter was updated
    pub last_updated_at: i64,
    /// _padding_2, previous whitelisted_wallet, BE CAREFUL FOR TOMBSTONE WHEN REUSE !!
    pub _padding_2: [u8; 32],
    /// Address allowed to swap when the current point is greater than or equal to the pre-activation point. The pre-activation point is calculated as `activation_point - pre_activation_duration`.
    pub pre_activation_swap_address: Pubkey,
    /// Base keypair. Only required for permission pair
    pub base_key: Pubkey,
    /// Time point to enable the pair. Only applicable for permission pair.
    pub activation_point: u64,
    /// Duration before activation point. Used to calculate pre-activation point for pre_activation_swap_address
    pub pre_activation_duration: u64,
    /// _padding 3 is reclaimed free space from swap_cap_deactivate_point and swap_cap_amount before, BE CAREFUL FOR TOMBSTONE WHEN REUSE !!
    pub _padding_3: [u8; 8],
    /// _padding_4, previous lock_duration, BE CAREFUL FOR TOMBSTONE WHEN REUSE !!
    pub _padding_4: u64,
    /// Pool creator
    pub creator: Pubkey,
    /// Reserved space for future use
    pub _reserved: [u8; 24],
}

impl LbPair {
    pub async fn from_rpc(rpc_client: &RpcClient, lb_pair_addr: &str) -> Result<Self> {
        let pubkey = Pubkey::from_str(lb_pair_addr)?;
        let account = rpc_client.get_account(&pubkey).await?;

        let result: LbPair = borsh::from_slice(&account.data)
            .map_err(|err| anyhow::anyhow!("deserialize meteora dlmm lbpair error: {err}"))?;

        Ok(result)
    }
}
