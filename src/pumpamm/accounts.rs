use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct PumpAmmPool {
    pub pool_bump: u8,
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub lp_supply: u64,
}
