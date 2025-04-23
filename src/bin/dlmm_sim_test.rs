use rust_decimal::{Decimal, MathematicalOps};

const BASIS_POINT_MAX: u64 = 10000;
const TOKEN_DECIMALS: u8 = 6;
const WSOL_DECIMALS: u8 = 9;

fn main() {
    let amm_init_wsol: u64 = 79 * 1_000_000_000;
    let amm_init_token: u64 = 200_000_000 * 1_000_000;
    let amm_init_price = Decimal::from(amm_init_wsol) / Decimal::from(amm_init_token);
    println!("amm init price: {}", amm_init_price);

    let bin_step = 400i32;
    let position_width = 70;

    let start_bin_id = -270i32;
    let start_price = price_of_bin(start_bin_id, bin_step);
    println!("start bin {start_bin_id} price is: {start_price}");

    let end_bin_id = start_bin_id + position_width;
    let end_price = price_of_bin(end_bin_id, bin_step);
    println!("end bin {end_bin_id} price is: {end_price}");
}

fn price_of_bin(bin_id: i32, bin_step: i32) -> Decimal {
    let bin_step_num = Decimal::from(bin_step) / Decimal::from(BASIS_POINT_MAX);
    (Decimal::from(1) + bin_step_num).powd(Decimal::from(bin_id))
}

fn price_per_token(price: Decimal) -> Decimal {
    let decimals_diff = Decimal::from(TOKEN_DECIMALS) - Decimal::from(WSOL_DECIMALS);
    price * (Decimal::from(10).powd(decimals_diff))
}
