use anyhow::Result;
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct MeteoraDlmmSwapEvent {
    pub evt_id: u64,
    // Liquidity pool pair
    pub lb_pair: Pubkey,
    // Address initiated the swap
    pub from: Pubkey,
    // Initial active bin ID
    pub start_bin_id: i32,
    // Finalized active bin ID
    pub end_bin_id: i32,
    // In token amount
    pub amount_in: u64,
    // Out token amount
    pub amount_out: u64,
    // Direction of the swap
    pub swap_for_y: bool,
    // Include protocol fee
    pub fee: u64,
    // Part of fee
    pub protocol_fee: u64,
    // Fee bps
    pub fee_bps: u128,
    // Host fee
    pub host_fee: u64,
}

impl MeteoraDlmmSwapEvent {
    pub fn from_cpi_log(log: &str) -> Result<Self> {
        let bytes = bs58::decode(log).into_vec()?;
        let evt: MeteoraDlmmSwapEvent = borsh::from_slice(&bytes[8..])?;
        Ok(evt)
    }
}

#[cfg(test)]
mod tests {
    use super::MeteoraDlmmSwapEvent;

    #[test]
    fn test_decode_swap_evt() {
        let evt_data = "yCGxBopjnVNQkNP5usq1PpLuVb2NpVsU6W7oHk1uLCBqSbdXeht3CBJqM9Tqo5eD8dWs3PcBsosJs4TvgcKDL59evdyxbk1yUH1Wjk81pBm4JBZyfTH9W4PNhbdf8ueHGDkFqhaW75JUGhrwv3T8GbkzpnbdFCFKdcT1gYQnH89AVpBPWqGU63e6nFFRBtTWASyZwM";

        let evt = MeteoraDlmmSwapEvent::from_cpi_log(evt_data).unwrap();
        println!("meteora dlmm swap event: {evt:#?}");
    }
}
