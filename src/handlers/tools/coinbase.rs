use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use super::{block::Block, reward::Reward, transaction::{Output, Transaction}, MerkelRoot};

//fees are sum of transactions fees
//relay's fee is 10% of fees
//validator fee is 90% of fees
//reward is only for validator
//merkel is merkel root of transactions
//size is number of transactions
//hash make from output
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Coinbase {
    pub hash: String,
    size: u8,
    pub merkel: String,
    #[serde_as(as = "DisplayFromStr")]
    pub reward: Decimal,
    pub output: Output,
    #[serde_as(as = "DisplayFromStr")]
    fees: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    relay_fee: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    validator_fee: Decimal,
}

impl Coinbase {
    //validating coinbase trx in a block that recieve
    pub async fn validation<'a>(
        &self,
        last_block: &mut Vec<Block>,
        transactions: &Vec<Transaction>,
    ) -> Result<(), &'a str> {
        let reward = Reward::calculate(last_block);
        if self.reward == reward {
            //make merkel root of block's transactions
            let mut trx_hashes = Vec::new();
            for t in transactions {
                trx_hashes.push(&t.hash);
            }
            let merkel = MerkelRoot::make(trx_hashes);

            //check merkel root that maked with coinbase merkel root to validation
            if merkel[0] == self.merkel {
                //calculate fees
                let fees: Decimal = transactions.iter().map(|trx| trx.fee).sum();
                let relay_fee = fees * Decimal::from_str("0.10").unwrap();
                let validator_fee = fees - relay_fee;

                //if fees was correct check outputs of coinbase for validate relay fee
                if fees == self.fees
                    && relay_fee == self.relay_fee
                    && validator_fee == self.validator_fee
                {
                    // if relay's fee was greater that 0 then check outputs of coinbase to see relay's fee
                    //if there is no any output for relay's fee coinbase will be rejected
                    if relay_fee > Decimal::from_str("0.0").unwrap() {
                        let mut relay_fee_check: Option<bool> = None;
                        for unspent in &self.output.unspents {
                            if unspent.data.value == relay_fee {
                                relay_fee_check.get_or_insert(true);
                            }
                        }
                        if relay_fee_check.unwrap() {
                            Ok(())
                        } else {
                            Err("Coinbase's relay fee is wrong!")
                        }
                    } else {
                        Ok(())
                    }
                } else {
                    Err("fees of coinbase transaction is wrong!")
                }
            } else {
                Err("Merkel root of coinbase is wrong!")
            }
        } else {
            Err("Recieved block's reward in coinbase is incorrect!")
        }
    }
}
