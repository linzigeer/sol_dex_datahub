use anyhow::Result;
use borsh::BorshDeserialize;
use serde::Serialize;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use super::PUMPFUN_PROGRAM_ID;

#[derive(Debug, Clone, Copy, BorshDeserialize, Serialize)]
pub struct BondingCurveAccount {
    pub discriminator: u64,
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,
}

#[allow(unused)]
impl BondingCurveAccount {
    pub fn find_pda(mint: Pubkey) -> Pubkey {
        let (pda, _) = Pubkey::find_program_address(
            &[
                &[98, 111, 110, 100, 105, 110, 103, 45, 99, 117, 114, 118, 101],
                &mint.to_bytes(),
            ],
            &PUMPFUN_PROGRAM_ID,
        );

        pda
    }

    pub async fn from_rpc(rpc_client: &RpcClient, curve: &Pubkey) -> Result<Self> {
        let account = rpc_client.get_account(curve).await?;

        let result: BondingCurveAccount = borsh::from_slice(&account.data)?;
        Ok(result)
    }
}
