use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;

pub mod accounts;
pub mod event;

pub const PUMPFUN_PROGRAM_ID: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const CREATE_LOG_PREFIX: &str = "Program data: G3KpTd7r";
pub const SWAP_LOG_PREFIX: &str = "Program data: vdt/007m";
pub const COMPLETE_LOG_PREFIX: &str = "Program data: X3JhnNQu";
pub const SETPARAMS_LOG_PREFIX: &str = "Program data: 38Of9j4w";

pub fn is_pumpfun_log(log: &str) -> bool {
    log.starts_with(SWAP_LOG_PREFIX)
        || log.starts_with(CREATE_LOG_PREFIX)
        || log.starts_with(COMPLETE_LOG_PREFIX)
        || log.starts_with(SETPARAMS_LOG_PREFIX)
}
