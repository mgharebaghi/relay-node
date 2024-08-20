use rust_decimal::Decimal;
use std::str::FromStr;

use super::block::Block;

pub struct Reward;

impl Reward {
    pub fn calculate(last_block: &mut Vec<Block>) -> Decimal {
        let reward: Decimal;
        if last_block[0].header.number % 1500000 == 0 {
            reward =
                last_block[0].body.coinbase.reward / Decimal::from_str("2.0").unwrap().round_dp(12);
            reward
        } else {
            reward = last_block[0].body.coinbase.reward;
            reward
        }
    }
}