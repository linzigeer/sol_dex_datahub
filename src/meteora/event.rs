use anyhow::Result;
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct MeteoraDlmmSwapEvent {
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

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct MeteoraLbPairCreateEvent {
    // Liquidity pool pair
    pub lb_pair: Pubkey,
    // Bin step
    pub bin_step: u16,
    // Address of token X
    pub token_x: Pubkey,
    // Address of token Y
    pub token_y: Pubkey,
}

#[derive(Debug)]
pub enum MeteoraDlmmEvents {
    Swap(MeteoraDlmmSwapEvent),
    LbPairCreate(MeteoraLbPairCreateEvent),
}

impl MeteoraDlmmEvents {
    pub fn from_cpi_log(log: &str) -> Result<Self> {
        let bytes = bs58::decode(log).into_vec()?;
        let bytes = &bytes[8..];

        let result = match &bytes[..8] {
            [81, 108, 227, 190, 205, 208, 10, 196] => {
                let evt: MeteoraDlmmSwapEvent = borsh::from_slice(&bytes[8..])?;
                Self::Swap(evt)
            }
            [185, 74, 252, 125, 27, 215, 188, 111] => {
                let evt: MeteoraLbPairCreateEvent = borsh::from_slice(&bytes[8..])?;
                Self::LbPairCreate(evt)
            }
            _ => anyhow::bail!("log is not recognized as meteora dlmm log: {log}"),
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_swap_evt() {
        let evt_data = "yCGxBopjnVNQkNP5usq1PpLuVb2NpVsU6W7oHk1uLCBqSbdXeht3CBJqM9Tqo5eD8dWs3PcBsosJs4TvgcKDL59evdyxbk1yUH1Wjk81pBm4JBZyfTH9W4PNhbdf8ueHGDkFqhaW75JUGhrwv3T8GbkzpnbdFCFKdcT1gYQnH89AVpBPWqGU63e6nFFRBtTWASyZwM";
        let evt = MeteoraDlmmEvents::from_cpi_log(evt_data).unwrap();
        println!("meteora dlmm swap event: {evt:#?}");
    }

    #[test]
    fn test_decode_lbpair_created_evt() {
        let evt_data = "FPwodQBxG1zfFUeFeUF2VDpU7KqWxHbyuYpoFzxe5t5Qaah8zV77xFwXU3wqndwXXp9N83wCyPtQMc9zS1xK4ithJuMsrt1sd9fe8MXr7fvPwciaSDTA2ZSPr49S41rui4adqcDb6a14uQcEz6vgJg9tpGeU";

        let evt = MeteoraDlmmEvents::from_cpi_log(evt_data).unwrap();
        println!("meteora dlmm lb pair created event: {evt:#?}");
    }
}
