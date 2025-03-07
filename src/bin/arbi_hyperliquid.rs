use alloy_core::primitives::U256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct QnEvmTxReceipt {
    pub block_hash: String,
    // pub block_number: U256,
}

pub struct QnEvmTx {}

fn main() {
    todo!();
}
