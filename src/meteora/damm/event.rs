use anyhow::Result;
use base64::{Engine, prelude::BASE64_STANDARD};
use borsh::BorshDeserialize;
use solana_sdk::pubkey::Pubkey;

use super::MeteoraDammPoolType;

#[derive(Debug, BorshDeserialize)]
pub struct MeteoraDammSwap {
    /// Token amount user deposited to the pool for token exchange.
    pub in_amount: u64,
    /// Token amount user received from the pool.
    pub out_amount: u64,
    /// Trading fee charged for liquidity provider.
    pub trade_fee: u64,
    /// Trading fee charged for the protocol.
    pub protocol_fee: u64,
    /// Host fee charged
    pub host_fee: u64,
}

#[derive(Debug, BorshDeserialize)]
pub struct MeteoraDammPoolCreated {
    /// LP token mint of the pool
    pub lp_mint: Pubkey, //32
    /// Token A mint of the pool. Eg: USDT
    pub token_a_mint: Pubkey, //32
    /// Token B mint of the pool. Eg: USDC
    pub token_b_mint: Pubkey, //32
    /// Pool type
    pub pool_type: MeteoraDammPoolType,
    /// Pool address
    pub pool: Pubkey,
}

#[derive(Debug)]
pub enum MeteoraDammEvents {
    Swap(MeteoraDammSwap),
    PoolCreated(MeteoraDammPoolCreated),
}

impl MeteoraDammEvents {
    pub fn from_log(log: &str) -> Result<Self> {
        let bytes = BASE64_STANDARD.decode(log)?;

        let result = match &bytes[..8] {
            [81, 108, 227, 190, 205, 208, 10, 196] => {
                let evt: MeteoraDammSwap = borsh::from_slice(&bytes[8..])?;
                Self::Swap(evt)
            }
            [202, 44, 41, 88, 104, 220, 157, 82] => {
                let evt: MeteoraDammPoolCreated = borsh::from_slice(&bytes[8..])?;
                Self::PoolCreated(evt)
            }
            _ => anyhow::bail!("log is not recognized as meteora damm log: {log}"),
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    pub fn test_decode_swap_evt() {
        // let encoded_evt = "UWzjvs3QCsSAwQAAAAAAALajBAAAAAAAYwAAAAAAAAAYAAAAAAAAAAAAAAAAAAAA";
        // let encoded_evt = "UWzjvs3QCsRgFrzf0kEAAJjIHA0AAAAAZAiXzoYAAAAYwqWzIQAAAAAAAAAAAAAA";
        // let encoded_evt = "UWzjvs3QCsQjDDUAAAAAAAZBwEcDAAAAmCAAAAAAAAAlCAAAAAAAAAAAAAAAAAAA";
        // let encoded_evt = "UWzjvs3QCsT/4PUFAAAAABgCqAQAAAAAECcAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        // let encoded_evt = "UWzjvs3QCsT04gtUAgAAABZs3AEAAAAAAC0xAQAAAABAS0wAAAAAAAAAAAAAAAAA";
        let encoded_evt = "UWzjvs3QCsSuVepPAAAAAPbFLwAAAAAArKqjAAAAAACr6igAAAAAAAAAAAAAAAAA";
        let evt_bytes = BASE64_STANDARD.decode(encoded_evt).unwrap();
        println!("swap discriminator: {:?}", &evt_bytes[..8]);

        let swap_evt = MeteoraDammEvents::from_log(encoded_evt).unwrap();
        println!("swap evt: {swap_evt:#?}");
    }
    #[test]
    pub fn test_decode_create_pool_evt() {
        let encoded_evt = "yiwpWGjcnVL/OEim1tJaIYv+uaPx+ExHNdLj9kYFNHhSYEHp3UqzpOXozgM2rUsMJx7iRsc7tS5W0xZVIVrmfBDwo4cZ855TBpuIV/6rgYT7aH9jRhjANdrEOdwa6ztVmKDwAAAAAAEBsLGkRP0LBqwdp+4Q412IQMSZjqfRwFJ5w7XpeoA2jvI=";
        let evt_bytes = BASE64_STANDARD.decode(encoded_evt).unwrap();
        println!("pool created discriminator: {:?}", &evt_bytes[..8]);

        let created_evt = MeteoraDammEvents::from_log(encoded_evt).unwrap();
        println!("pool created evt: {created_evt:#?}");
    }
}
