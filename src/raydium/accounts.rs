use std::str::FromStr;

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed)]
pub struct Fees {
    /// numerator of the min_separate
    pub min_separate_numerator: u64,
    /// denominator of the min_separate
    pub min_separate_denominator: u64,

    /// numerator of the fee
    pub trade_fee_numerator: u64,
    /// denominator of the fee
    /// and 'trade_fee_denominator' must be equal to 'min_separate_denominator'
    pub trade_fee_denominator: u64,

    /// numerator of the pnl
    pub pnl_numerator: u64,
    /// denominator of the pnl
    pub pnl_denominator: u64,

    /// numerator of the swap_fee
    pub swap_fee_numerator: u64,
    /// denominator of the swap_fee
    pub swap_fee_denominator: u64,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed)]
pub struct StateData {
    /// delay to take pnl coin
    pub need_take_pnl_coin: u64,
    /// delay to take pnl pc
    pub need_take_pnl_pc: u64,
    /// total pnl pc
    pub total_pnl_pc: u64,
    /// total pnl coin
    pub total_pnl_coin: u64,
    /// ido pool open time
    pub pool_open_time: u64,
    /// padding for future updates
    pub padding: [u64; 2],
    /// switch from order book only to init
    pub orderbook_to_init_time: u64,

    /// swap coin in amount
    pub swap_coin_in_amount: u128,
    /// swap pc out amount
    pub swap_pc_out_amount: u128,
    /// charge pc as swap fee while swap pc to coin
    pub swap_acc_pc_fee: u64,

    /// swap pc in amount
    pub swap_pc_in_amount: u128,
    /// swap coin out amount
    pub swap_coin_out_amount: u128,
    /// charge coin as swap fee while swap coin to pc
    pub swap_acc_coin_fee: u64,
}

#[derive(Debug, Copy, Clone, Default, Pod, Zeroable)]
#[repr(C, packed)]
pub struct AmmInfo {
    /// Initialized status.
    pub status: u64,
    /// Nonce used in program address.
    /// The program address is created deterministically with the nonce,
    /// amm program id, and amm account pubkey.  This program address has
    /// authority over the amm's token coin account, token pc account, and pool
    /// token mint.
    pub nonce: u64,
    /// max order count
    pub order_num: u64,
    /// within this range, 5 => 5% range
    pub depth: u64,
    /// coin decimal
    pub coin_decimals: u64,
    /// pc decimal
    pub pc_decimals: u64,
    /// amm machine state
    pub state: u64,
    /// amm reset_flag
    pub reset_flag: u64,
    /// min size 1->0.000001
    pub min_size: u64,
    /// vol_max_cut_ratio numerator, sys_decimal_value as denominator
    pub vol_max_cut_ratio: u64,
    /// amount wave numerator, sys_decimal_value as denominator
    pub amount_wave: u64,
    /// coinLotSize 1 -> 0.000001
    pub coin_lot_size: u64,
    /// pcLotSize 1 -> 0.000001
    pub pc_lot_size: u64,
    /// min_cur_price: (2 * amm.order_num * amm.pc_lot_size) * max_price_multiplier
    pub min_price_multiplier: u64,
    /// max_cur_price: (2 * amm.order_num * amm.pc_lot_size) * max_price_multiplier
    pub max_price_multiplier: u64,
    /// system decimal value, used to normalize the value of coin and pc amount
    pub sys_decimal_value: u64,
    /// All fee information
    pub fees: Fees,
    /// Statistical data
    pub state_data: StateData,
    /// Coin vault
    pub coin_vault: Pubkey,
    /// Pc vault
    pub pc_vault: Pubkey,
    /// Coin vault mint
    pub coin_vault_mint: Pubkey,
    /// Pc vault mint
    pub pc_vault_mint: Pubkey,
    /// lp mint
    pub lp_mint: Pubkey,
    /// open_orders key
    pub open_orders: Pubkey,
    /// market key
    pub market: Pubkey,
    /// market program key
    pub market_program: Pubkey,
    /// target_orders key
    pub target_orders: Pubkey,
    /// padding
    pub padding1: [u64; 8],
    /// amm owner key
    pub amm_owner: Pubkey,
    /// pool lp amount
    pub lp_amount: u64,
    /// client order id
    pub client_order_id: u64,
    /// recent epoch
    pub recent_epoch: u64,
    /// padding
    pub padding2: u64,
}

impl AmmInfo {
    pub async fn from_rpc(rpc_client: &RpcClient, amm_addr: &str) -> Result<Self> {
        let pubkey = Pubkey::from_str(amm_addr)?;
        let account = rpc_client.get_account(&pubkey).await?;

        let result: &AmmInfo = bytemuck::checked::try_from_bytes::<AmmInfo>(&account.data)
            .map_err(|err| anyhow::anyhow!("deserialize amm info error: {err}"))?;

        Ok(*result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn test_parse_amm() {
        let data = "BgAAAAAAAAD+AAAAAAAAAAcAAAAAAAAAAwAAAAAAAAAJAAAAAAAAAAkAAAAAAAAAAQAAAAAAAAAAAAAAAAAAAADKmjsAAAAA9AEAAAAAAABAS0wAAAAAAADKmjsAAAAA6AMAAAAAAAABAAAAAAAAAADKmjsAAAAAAMqaOwAAAAAFAAAAAAAAABAnAAAAAAAAGQAAAAAAAAAQJwAAAAAAAAwAAAAAAAAAZAAAAAAAAAAZAAAAAAAAABAnAAAAAAAAAAAAAAAAAAAAAAAAAAAAAPORJhkAAAAAdkjr8YQjAwA8UoVmAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA9xR8a0M5ADAAAAAAAAAADW9H0/RwAAAAAAAAAAAAAAfP3mLQAAAADQBey4RwAAAAAAAAAAAAAAC0iE/dmjmAMAAAAAAAAAAEkIBCzPRwIAejrpcSqpRKi+BLMtwPHYpYEqW8xMfSjaJYdbtz28Hgdcgiz99inzJYnzxNjMPM7agwHTv+J+7T0Hr3a26Gwhjw0mJMQYvZ2Bq+3tgdwUhCBZtuY5+s1FfygG7yrNRY0kBpuIV/6rgYT7aH9jRhjANdrEOdwa6ztVmKDwAAAAAAEBkiNxbpQYWuYiBek+9wGuwZAmftW8jNpzlWx/g6Eadi/nPyalUaf8g71z7DZMjg1ClJp+hYcyFENe7RRInqzMHXJjIuOA/eNOTBJMbQttBOXRnXTxzMOIGNE3Pq4WH68NB1GoKC2mEwX+KZw3uZjlhHHbETUDcxD4vhBFpgr27hlI6lfThLCSMCeHy4+EMcqr1VVjC0Trf/dF3BdayTZ3AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAOW2K2XLO72m9WiI5m/ujmTcVWAZnA+IsR/ic70FnoqhAMqaOwAAAAAAAAAAAAAAAMICAAAAAAAAAAAAAAAAAAA=";
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .unwrap();

        let amm_info: AmmInfo = *bytemuck::checked::from_bytes(&bytes);
        println!("amm info: {amm_info:#?}");
    }
}
