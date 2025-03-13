#![allow(unused)]
use anyhow::Result;
use borsh::BorshDeserialize;
use solana_sdk::borsh1;
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
                let evt: TradeEvent = borsh1::try_from_slice_unchecked(bytes)?;
                Self::Trade(evt)
            }
            [27, 114, 169, 77, 222, 235, 99, 118] => {
                let evt: CreateEvent = borsh1::try_from_slice_unchecked(bytes)?;
                Self::Create(evt)
            }
            [95, 114, 97, 156, 212, 46, 152, 8] => {
                let evt: CompleteEvent = borsh1::try_from_slice_unchecked(bytes)?;
                Self::Complete(evt)
            }
            [223, 195, 159, 246, 62, 48, 143, 131] => {
                let evt: SetParamsEvent = borsh1::try_from_slice_unchecked(bytes)?;
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
    use base64::prelude::BASE64_STANDARD;

    use super::*;

    #[test]
    fn test_decode_pump_trade_evt() {
        let encoded_evt = "2K7nL28PxCW8ejnyCeuMpbXwJKzXo9q1ecEyRsXKe7VYaxLjCqTrMCp9pnwrwTG7rmaRTa1vcTqa8LGDfNZ9bpcKgSPgNDe3MrFn57HPpTzriKWACnH99YDM7dfTpxwRoCQTrs6BSdGSXgusW9Jbz1yAV9D32MZ62azsiK16Gksbq7cinYkugTfQDJM5";
        let evt = PumpFunEvents::from_cpi_log(encoded_evt).unwrap();
        println!("pumpfun trade event: {evt:#?}");
    }

    #[test]
    fn test_decode_pump_create_evt() {
        let encode_created_evt = "3ck7szVsdFfNhc7Yijezdmy73fWycmttUN6UNb1vQjPYZxr67fnmDnC2MgoRbX4RAzyCtqLwnaKqkRfyCF34WAB9Wxsm1aojum6cU4aMuUKwnuDzE39zoQV1G36mGdwspN52tiueFdcB7CMNK1ejYzzdM6ppYRK1Miay5UirZTWuNZESJz5Ci9smPWQoRvftDYvciK7WYg4TcVkteadFBcMzywKFWBhwshyyzc6cMv1brCM3G5nVNycLKtVJkwcnfLaLCz469dhdyZ9PARNfvSiGHZ74GBJecXq8BYu3Nmh36hB3Qt3fnbdvQFhCtkCD68ziVTzy8XbvedYsRvgijDSJXTU1h8FPzzebXXwKzgrb";
        let evt = PumpFunEvents::from_cpi_log(encode_created_evt).unwrap();
        println!("pumpfun create event: {evt:#?}");
    }

    #[test]
    fn test_decode_pump_compete_evt() {
        let encoded_complete_evt = "YeADJEDSy5WzCFuDLrfFZ2pQG5GsJCGudQvZj1RHwD74UBRabt1MxxGPoTRn432WCj9Vf1P127Qp6qABSeNoFzvj4XikFhDkePCMjuTk178GtBLsbaKC7tt4yJvwcQnuY7bSqHLsyadheV3Z4YJjPnbPJ6PBMXrvEyMZ";
        let evt = PumpFunEvents::from_cpi_log(encoded_complete_evt).unwrap();
        println!("pumpfun complete event: {evt:#?}");
    }
}
