use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;

use super::MeteoraDammPoolType;

#[derive(Copy, Clone, Debug, BorshDeserialize)]
pub struct PoolFees {
    /// Trade fees are extra token amounts that are held inside the token
    /// accounts during a trade, making the value of liquidity tokens rise.
    /// Trade fee numerator
    pub trade_fee_numerator: u64,
    /// Trade fee denominator
    pub trade_fee_denominator: u64,

    /// Owner trading fees are extra token amounts that are held inside the token
    /// accounts during a trade, with the equivalent in pool tokens minted to
    /// the owner of the program.
    /// Owner trade fee numerator
    pub protocol_trade_fee_numerator: u64,
    /// Owner trade fee denominator
    pub protocol_trade_fee_denominator: u64,
}

#[derive(Copy, Clone, Debug, Default, BorshDeserialize)]
pub struct Bootstrapping {
    /// Activation point, can be slot or timestamp
    pub activation_point: u64,
    /// Whitelisted vault to be able to buy pool before open slot
    pub whitelisted_vault: Pubkey,
    #[deprecated]
    pub pool_creator: Pubkey,
    /// Activation type, 0 means by slot, 1 means by timestamp
    pub activation_type: u8,
}

#[derive(BorshDeserialize, Clone, Debug, Default, Copy, Eq, PartialEq)]
/// Multiplier for the pool token. Used to normalized token with different decimal into the same precision.
pub struct TokenMultiplier {
    /// Multiplier for token A of the pool.
    pub token_a_multiplier: u64, // 8
    /// Multiplier for token B of the pool.
    pub token_b_multiplier: u64, // 8
    /// Record the highest token decimal in the pool. For example, Token A is 6 decimal, token B is 9 decimal. This will save value of 9.
    pub precision_factor: u8, // 1
}

/// Type of depeg pool
#[derive(Clone, Copy, Debug, Default, BorshDeserialize, PartialEq)]
pub enum DepegType {
    #[default]
    /// Indicate that it is not a depeg pool
    None,
    /// A depeg pool belongs to marinade finance
    Marinade,
    /// A depeg pool belongs to solido
    Lido,
    /// A depeg pool belongs to SPL stake pool program
    SplStake,
}

/// Contains information for depeg pool
#[derive(Clone, Copy, Debug, Default, BorshDeserialize)]
pub struct Depeg {
    /// The virtual price of staking / interest bearing token
    pub base_virtual_price: u64,
    /// The virtual price of staking / interest bearing token
    pub base_cache_updated: u64,
    /// Type of the depeg pool
    pub depeg_type: DepegType,
}

#[derive(Clone, Copy, Debug, BorshDeserialize)]
/// Type of the swap curve
pub enum CurveType {
    /// Uniswap-style constant product curve, invariant = token_a_amount * token_b_amount
    ConstantProduct,
    /// Stable, like uniswap, but with wide zone of 1:1 instead of one point
    Stable {
        /// Amplification coefficient
        amp: u64,
        /// Multiplier for the pool token. Used to normalized token with different decimal into the same precision.
        token_multiplier: TokenMultiplier,
        /// Depeg pool information. Contains functions to allow token amount to be repeg using stake / interest bearing token virtual price
        depeg: Depeg,
        /// The last amp updated timestamp. Used to prevent update_curve_info called infinitely many times within a short period
        last_amp_updated_timestamp: u64,
    },
}

#[derive(Copy, Clone, Debug, BorshDeserialize, Default)]
pub struct PartnerInfo {
    pub fee_numerator: u64,
    pub partner_authority: Pubkey,
    pub pending_fee_a: u64,
    pub pending_fee_b: u64,
}

#[derive(BorshDeserialize, Default, Debug, Clone, Copy)]
pub struct Padding {
    /// Padding 0
    pub padding_0: [u8; 6], // 6
    /// Padding 1
    pub padding_1: [u64; 21], // 168
    /// Padding 2
    pub padding_2: [u64; 21], // 168
}

#[derive(Debug, BorshDeserialize)]
/// State of pool account
pub struct MeteoraDammPool {
    pub d: u64,
    /// LP token mint of the pool
    pub lp_mint: Pubkey, //32
    /// Token A mint of the pool. Eg: USDT
    pub token_a_mint: Pubkey, //32
    /// Token B mint of the pool. Eg: USDC
    pub token_b_mint: Pubkey, //32
    /// Vault account for token A. Token A of the pool will be deposit / withdraw from this vault account.
    pub a_vault: Pubkey, //32
    /// Vault account for token B. Token B of the pool will be deposit / withdraw from this vault account.
    pub b_vault: Pubkey, //32
    /// LP token account of vault A. Used to receive/burn the vault LP upon deposit/withdraw from the vault.
    pub a_vault_lp: Pubkey, //32
    /// LP token account of vault B. Used to receive/burn the vault LP upon deposit/withdraw from the vault.
    pub b_vault_lp: Pubkey, //32
    /// "A" vault lp bump. Used to create signer seeds.
    pub a_vault_lp_bump: u8, //1
    /// Flag to determine whether the pool is enabled, or disabled.
    pub enabled: bool, //1
    /// Protocol fee token account for token A. Used to receive trading fee.
    pub protocol_token_a_fee: Pubkey, //32
    /// Protocol fee token account for token B. Used to receive trading fee.
    pub protocol_token_b_fee: Pubkey, //32
    /// Fee last updated timestamp
    pub fee_last_updated_at: u64,
    // Padding leftover from deprecated admin pubkey. Beware of tombstone when reusing it.
    pub _padding0: [u8; 24],
    /// Store the fee charges setting.
    pub fees: PoolFees, //48
    /// Pool type
    pub pool_type: MeteoraDammPoolType,
    /// Stake pubkey of SPL stake pool
    pub stake: Pubkey,
    /// Total locked lp token
    pub total_locked_lp: u64,
    /// Bootstrapping config
    pub bootstrapping: Bootstrapping,
    pub partner_info: PartnerInfo,
    /// Padding for future pool field
    pub padding: Padding,
    /// The type of the swap curve supported by the pool.
    // Leaving curve_type as last field give us the flexibility to add specific curve information / new curve type
    pub curve_type: CurveType, //9
}
