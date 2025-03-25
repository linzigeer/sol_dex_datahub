use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// LogType enum
#[derive(Debug)]
pub enum LogType {
    Init,
    Deposit,
    Withdraw,
    SwapBaseIn,
    SwapBaseOut,
}

#[allow(unused)]
impl LogType {
    pub fn from_u8(log_type: u8) -> Self {
        match log_type {
            0 => LogType::Init,
            1 => LogType::Deposit,
            2 => LogType::Withdraw,
            3 => LogType::SwapBaseIn,
            4 => LogType::SwapBaseOut,
            _ => unreachable!(),
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            LogType::Init => 0u8,
            LogType::Deposit => 1u8,
            LogType::Withdraw => 2u8,
            LogType::SwapBaseIn => 3u8,
            LogType::SwapBaseOut => 4u8,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InitLog {
    pub log_type: u8,
    pub time: u64,
    pub pc_decimals: u8,
    pub coin_decimals: u8,
    pub pc_lot_size: u64,
    pub coin_lot_size: u64,
    pub pc_amount: u64,
    pub coin_amount: u64,
    pub market: Pubkey,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DepositLog {
    pub log_type: u8,
    // input
    pub max_coin: u64,
    pub max_pc: u64,
    pub base: u64,
    // pool info
    pub pool_coin: u64,
    pub pool_pc: u64,
    pub pool_lp: u64,
    pub calc_pnl_x: u128,
    pub calc_pnl_y: u128,
    // calc result
    pub deduct_coin: u64,
    pub deduct_pc: u64,
    pub mint_lp: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WithdrawLog {
    pub log_type: u8,
    // input
    pub withdraw_lp: u64,
    // user info
    pub user_lp: u64,
    // pool info
    pub pool_coin: u64,
    pub pool_pc: u64,
    pub pool_lp: u64,
    pub calc_pnl_x: u128,
    pub calc_pnl_y: u128,
    // calc result
    pub out_coin: u64,
    pub out_pc: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SwapBaseInLog {
    pub log_type: u8,
    // input
    pub amount_in: u64,
    pub minimum_out: u64,
    pub direction: u64,
    // user info
    pub user_source: u64,
    // pool info
    pub pool_coin: u64,
    pub pool_pc: u64,
    // calc result
    pub out_amount: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SwapBaseOutLog {
    pub log_type: u8,
    // input
    pub max_in: u64,
    pub amount_out: u64,
    pub direction: u64,
    // user info
    pub user_source: u64,
    // pool info
    pub pool_coin: u64,
    pub pool_pc: u64,
    // calc result
    pub deduct_in: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum RayLogs {
    Init(InitLog),
    Deposit(DepositLog),
    Withdraw(WithdrawLog),
    SwapBaseIn(SwapBaseInLog),
    SwapBaseOut(SwapBaseOutLog),
}

impl RayLogs {
    pub fn decode(log: &str) -> Result<Self> {
        let bytes = STANDARD.decode(log)?;
        let result = match LogType::from_u8(bytes[0]) {
            LogType::Init => {
                let log: InitLog = bincode::deserialize(&bytes)?;
                RayLogs::Init(log)
            }
            LogType::Deposit => {
                let log: DepositLog = bincode::deserialize(&bytes)?;
                RayLogs::Deposit(log)
            }
            LogType::Withdraw => {
                let log: WithdrawLog = bincode::deserialize(&bytes)?;
                RayLogs::Withdraw(log)
            }
            LogType::SwapBaseIn => {
                let log: SwapBaseInLog = bincode::deserialize(&bytes)?;
                RayLogs::SwapBaseIn(log)
            }
            LogType::SwapBaseOut => {
                let log: SwapBaseOutLog = bincode::deserialize(&bytes)?;
                RayLogs::SwapBaseOut(log)
            }
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_swap_basein() {
        let result = RayLogs::decode(
            "A1x8BAAAAAAAqgAAAAAAAAABAAAAAAAAAFx8BAAAAAAA4kxOVRsAAADq2uJNY4UAAOoAAAAAAAAA",
        )
        .unwrap();

        println!("{result:#?}");
        assert!(matches!(
            result,
            RayLogs::SwapBaseIn(SwapBaseInLog {
                log_type: 3,
                amount_in: 293980,
                minimum_out: 170,
                out_amount: 234,
                ..
            })
        ))
    }

    #[test]
    fn test_decode_withdraw() {
        let result = RayLogs::decode("Aowy0KQAAAAAjDLQpAAAAAAOVgk3AAAAAOn/ZSQQAAAA1yZyNwEAAABRxNj660cAAAAAAAAAAAAAxgFXLwAAAAAAAAAAAAAAAHLmHx0AAAAAZkDQiggAAAA=").unwrap();
        println!("{result:#?}");
        assert!(matches!(
            result,
            RayLogs::Withdraw(WithdrawLog {
                log_type: 2,
                out_coin: 488629874,
                out_pc: 36688642150,
                ..
            })
        ))
    }

    #[test]
    fn test_decode_init() {
        let result = RayLogs::decode("AMrTUGcAAAAABgkQJwAAAAAAAADKmjsAAAAAFCn1TAMAAAAAypo7AAAAABVwbJyjtAt7hWR5/LLLQauTYDcNHIrAZ8tELy5TTWd5").unwrap();
        println!("{result:#?}");
        assert!(matches!(
            result,
            RayLogs::Init(InitLog {
                log_type: 0,
                coin_amount: 1000000000,
                pc_amount: 14176037140,
                ..
            })
        ))
    }
}
