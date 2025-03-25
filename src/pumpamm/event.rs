use anyhow::Result;
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;
use tracing::{debug, warn};

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct PumpAmmCreatePoolEvent {
    pub timestamp: i64,
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_mint_decimals: u8,
    pub quote_mint_decimals: u8,
    pub base_amount_in: u64,
    pub quote_amount_in: u64,
    pub pool_base_amount: u64,
    pub pool_quote_amount: u64,
    pub minimum_liquidity: u64,
    pub initial_liquidity: u64,
    pub lp_token_amount_out: u64,
    pub pool_bump: u8,
    pub pool: Pubkey,
    pub lp_mint: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
}

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct PumpAmmBuyEvent {
    pub timestamp: i64,
    pub base_amount_out: u64,
    pub max_quote_amount_in: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub quote_amount_in: u64,
    pub lp_fee_basis_points: u64,
    pub lp_fee: u64,
    pub protocol_fee_basis_points: u64,
    pub protocol_fee: u64,
    pub quote_amount_in_with_lp_fee: u64,
    pub user_quote_amount_in: u64,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub protocol_fee_recipient: Pubkey,
    pub protocol_fee_recipient_token_account: Pubkey,
}

#[derive(Debug, Clone, Copy, BorshDeserialize)]
pub struct PumpAmmSellEvent {
    pub timestamp: i64,
    pub base_amount_in: u64,
    pub min_quote_amount_out: u64,
    pub user_base_token_reserves: u64,
    pub user_quote_token_reserves: u64,
    pub pool_base_token_reserves: u64,
    pub pool_quote_token_reserves: u64,
    pub quote_amount_out: u64,
    pub lp_fee_basis_points: u64,
    pub lp_fee: u64,
    pub protocol_fee_basis_points: u64,
    pub protocol_fee: u64,
    pub quote_amount_out_without_lp_fee: u64,
    pub user_quote_amount_out: u64,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub protocol_fee_recipient: Pubkey,
    pub protocol_fee_recipient_token_account: Pubkey,
}

#[derive(Debug)]
pub enum PumpAmmEvents {
    CreatePool(PumpAmmCreatePoolEvent),
    Buy(PumpAmmBuyEvent),
    Sell(PumpAmmSellEvent),
}

impl PumpAmmEvents {
    pub fn from_cpi_log(log: &str) -> Result<Self> {
        debug!("parse pumpamm log: {log}");
        let bytes = bs58::decode(log).into_vec()?;
        let bytes = &bytes[8..];

        let result = match &bytes[..8] {
            [177, 49, 12, 210, 160, 118, 167, 116] => {
                let evt: PumpAmmCreatePoolEvent = borsh::from_slice(&bytes[8..])?;
                Self::CreatePool(evt)
            }
            [103, 244, 82, 31, 44, 245, 119, 119] => {
                let evt: PumpAmmBuyEvent = borsh::from_slice(&bytes[8..])?;
                Self::Buy(evt)
            }
            [62, 47, 55, 10, 165, 3, 220, 42] => {
                let evt: PumpAmmSellEvent = borsh::from_slice(&bytes[8..])?;
                Self::Sell(evt)
            }
            _ => {
                let msg = format!("log is not recognized as pump amm log: {log}");
                warn!(msg);
                anyhow::bail!(msg)
            }
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_create_pool_evt() {
        let evt_data = "rLaD5MVJGTSekbeMDJ6HPu2vjcD1CxmDA1gQymYBcRq6XBB4xCkgHtGtWK2Q4cJCJaqU3cbnFFpYE1VuvorWUEyvmRvi3822c3tEnKFiNEkgEhy2eiGskn9DhuyyMPURFDGNQCMfqurSm39XCu5HRsKgPi8pWxrzpDf6XaAaw1F8ti4D2CDJCQU5wKUqiGTcUt5phxnyNHAx13V4YWW6RjU5yoY5aXFeE7vwhkPnVGdJSKFioPEydYHWJnXLydcvKL5w91kkPSCPeGtFhV1nJSHW8WV48x32xd3DQgHS8yyniBjbenhF7M9Lw7Nu1969mk71vKMhes8BzPN4tQbbBQNSeKfxRb3nqkiLKUFaSqezDDLsc1W6LJpv3rh1tKHd1CFEMeMoa73twgb73aZ7cem9mrV2cuutYtqsNr";
        let evt = PumpAmmEvents::from_cpi_log(evt_data).unwrap();
        println!("pump amm create pool event: {evt:#?}");
    }

    #[test]
    fn test_buy_evt() {
        let evt_data = "w1295DLPcEG5wn5ZTAu91vQ18djDpDL3tybTWvQVi2WRAVj2ozjJ175VoKUrAn3DL6fvGfri2FxUBCkCtQW1945U26ADQX8fEBMBgHySLwbXxZodRxUYB4hBfD5MJK3CU3i7Un2vmZAKjCGAjZXggLmCdPdN5BAUZVC2p793gzEAkvAF7uugNXHDJ1KWPWLj1f7HGcQEhUKEwZAumW9YoPWfikc3Rf22mA5KQNZkhbk4XbDuASKSarMEEmjnXcp3Sxo2RarcE5nBj8Vn73VdDsfAFBHzPqHrxQ9MU1Zka3cSupvF4iwH5Sz1DJ9Da97EQthDTX6nP2uHB3UemQobL5NJ1Sk5tL5Kp13dv1NhLCggsJ5HUCy5nSpGwYPniDyPUvMEL6peWf2V6jWuAQ6ctS4pPAnpT5eTKGKpeECae3cZ55ot62ErQ";

        let evt = PumpAmmEvents::from_cpi_log(evt_data).unwrap();
        println!("pump amm buy event: {evt:#?}");
    }

    #[test]
    fn test_sell_evt() {
        let evt_data = "w1295DLPcEFrZVGvC9FAJRzkesEEPkg7dr1Fip6zXypBg16aNJWJEi5ocDmYTrudzSikvC4HkiEfMpkYgHGPeZiVmAxrXDHyAjCQLoeYDSmTAgNXahrdmDcZvc2xzp5osdZwF3YJwkAw9Lx5MVwzeA6xgLEM1h2fXEXwLgZ3MtswS5WLKcZDKcogZa7rp29BdpjXUkAvCkbCFEiwTTNLSdyXo5eLRUUqco4dt3oaPcNqDqsyxRZZ9PMoh3pXHHFifQjtbX4uMLkepryCvZA9tF4GVhYGS4sm2wkDTZ6HrBroaqCt1uNfpK7MFmBDvKung5oLsUdJPFGutVLA9AHC1fnnR89fMRmwZpwf8T4jHR2GBCbJwDHS6pK1BkmBpKUoLyn7oC3wpdG8u98qzN7oSBZMNgXDfWdpq4cQFj814zC4gB49RDcWH";

        let evt = PumpAmmEvents::from_cpi_log(evt_data).unwrap();
        println!("pump amm sell event: {evt:#?}");
    }
}
