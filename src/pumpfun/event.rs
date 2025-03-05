#![allow(unused)]
use anyhow::Result;
use base64::Engine;
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use tracing::debug;

#[derive(Debug, BorshDeserialize)]
pub struct TradeEvent {
    pub discriminator: u64,
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
}

#[derive(Debug, BorshDeserialize)]
pub struct CreateEvent {
    pub discriminator: u64,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
}

#[derive(Debug, BorshDeserialize)]
pub struct CompleteEvent {
    pub discriminator: u64,
    pub user: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
}

#[derive(Debug, BorshDeserialize)]
pub struct SetParamsEvent {
    pub discriminator: u64,
    pub fee_recipient: Pubkey,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u64,
}

#[derive(Debug)]
pub enum PumpFunEvents {
    Trade(TradeEvent),
    Create(CreateEvent),
    Complete(CompleteEvent),
    SetParams(SetParamsEvent),
}

#[derive(Debug, PartialEq, Eq)]
pub enum PumpFunEventKind {
    Create,
    Buy,
    Sell,
    Complete,
    SetParams,
}

impl PumpFunEvents {
    pub fn from_cpi_log(log: &str) -> Result<Self> {
        let bytes = bs58::decode(log).into_vec()?;
        let bytes = &bytes[8..];

        let result = match &bytes[..8] {
            [189, 219, 127, 211, 78, 230, 97, 238] => {
                let evt: TradeEvent = borsh::from_slice(bytes)?;
                Self::Trade(evt)
            }
            [27, 114, 169, 77, 222, 235, 99, 118] => {
                let evt: CreateEvent = borsh::from_slice(bytes)?;
                Self::Create(evt)
            }
            [95, 114, 97, 156, 212, 46, 152, 8] => {
                let evt: CompleteEvent = borsh::from_slice(bytes)?;
                Self::Complete(evt)
            }
            [223, 195, 159, 246, 62, 48, 143, 131] => {
                let evt: SetParamsEvent = borsh::from_slice(bytes)?;
                Self::SetParams(evt)
            }
            _ => anyhow::bail!("log is not pumpfun log: {log}"),
        };

        Ok(result)
    }

    pub fn kind(&self) -> PumpFunEventKind {
        match self {
            PumpFunEvents::Trade(t) if t.is_buy => PumpFunEventKind::Buy,
            PumpFunEvents::Trade(_) => PumpFunEventKind::Sell,
            PumpFunEvents::Create(_) => PumpFunEventKind::Create,
            PumpFunEvents::Complete(_) => PumpFunEventKind::Complete,
            PumpFunEvents::SetParams(_) => PumpFunEventKind::SetParams,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_pump_evt() {
        let encoded_evt = "2K7nL28PxCW8ejnyCeuMpbXwJKzXo9q1ecEyRsXKe7VYaxLjCqTrMCp9pnwrwTG7rmaRTa1vcTqa8LGDfNZ9bpcKgSPgNDe3MrFn57HPpTzriKWACnH99YDM7dfTpxwRoCQTrs6BSdGSXgusW9Jbz1yAV9D32MZ62azsiK16Gksbq7cinYkugTfQDJM5";
        let evt = PumpFunEvents::from_cpi_log(encoded_evt).unwrap();
        println!("{evt:#?}");
    }
}
