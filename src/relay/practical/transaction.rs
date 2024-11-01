use chrono::Utc;
use libp2p::Swarm;
use mongodb::{
    bson::{doc, to_document, Document},
    Collection, Database,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use sp_core::{ed25519::Public, Pair};

use crate::relay::{
    events::connections::ConnectionsHandler,
    tools::{utxo::UTXO, HashMaker, MerkelRoot},
};

use super::{
    block::header::Sign,
    leader::{Leader, LeaderTime},
    swarm::CentichainBehaviour,
};

// Define a transaction in the Centichain network
// The hash of the transaction is derived from the hashes of its inputs and outputs
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Transaction {
    pub hash: String,
    pub input: Input,
    pub output: Output,
    #[serde_as(as = "DisplayFromStr")]
    pub value: Decimal,
    #[serde_as(as = "DisplayFromStr")]
    pub fee: Decimal,
    pub script: Script,
    pub signature: Vec<Sign>,
    pub date: String,
}

// Define a script for highlighting the transaction's signature
// It can have either a single signature or multiple signatures
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Script {
    Single,
    Multi,
}

// Define an input that includes UTXOs from other transactions' outputs, the number of UTXOs,
// and the hash of the UTXOs
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Input {
    hash: String,
    number: u8,
    utxos: Vec<UTXO>,
}

// Define an output that includes new UTXOs, the number of UTXOs, and the public key of the transaction creator
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Output {
    pub hash: String,
    pub number: usize,
    pub unspents: Vec<Unspent>,
}

//Define an unspent that has hash of unpsnet which is a new output for make a special UTXO
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Unspent {
    pub hash: String,
    pub data: UnspentData,
}
//Define a new output of a transaction that includes the wallet address,
// a salt for better hashing, and the unspent value derived from the transaction input
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct UnspentData {
    pub wallet: Public,
    pub salt: u32,
    #[serde_as(as = "DisplayFromStr")]
    pub value: Decimal,
}

impl Transaction {
    pub async fn validate<'a>(&self, db: &Database) -> Result<&Self, &'a str> {
        //make input and output hash to check hash that is correct or not
        let inputs_str = serde_json::to_string(&self.input.utxos).unwrap();
        let outputs_str = serde_json::to_string(&self.output.unspents).unwrap();
        let input_hash = HashMaker::generate(&inputs_str);
        let output_hash = HashMaker::generate(&outputs_str);

        //check input and output hash that is correct or not
        if input_hash == self.input.hash && output_hash == self.output.hash {
            //make tansaction's hash for check that it is correct or not
            let hashes = vec![&input_hash, &output_hash];
            let trx_hash = MerkelRoot::make(hashes);

            //check transaction hash
            if trx_hash[0] == self.hash {
                //validating signatrue of trx
                let sign_check = sp_core::ed25519::Pair::verify(
                    &self.signature[0].signatgure,
                    &trx_hash[0],
                    &self.signature[0].key,
                );

                //if validation done transaction is correct
                if sign_check {
                    //validating input utxos
                    let mut is_err: Option<&str> = None;
                    for i in 0..self.input.utxos.len() {
                        match UTXO::check(&self.input.utxos[i], db, &self.signature[0].key).await {
                            Ok(_) => {}
                            Err(e) => {
                                is_err = Some(e);
                                break;
                            }
                        }
                    }

                    //if inputs utxo doesn't have any problems return true
                    if is_err.is_none() {
                        Ok(self)
                    } else {
                        Err(is_err.unwrap())
                    }
                } else {
                    Err("Transaction is incorrect.(siganture problem!)")
                }
            } else {
                Err("Transaction is incorrect.(transacrtion hash problem!)")
            }
        } else {
            Err("Transaction is incorrect.(input/output hash problem!)")
        }
    }

    pub async fn insertion<'a>(
        &self,
        db: &'a Database,
        leader: &mut Leader,
        connection_handler: &mut ConnectionsHandler,
        swarm: &mut Swarm<CentichainBehaviour>,
    ) -> Result<(), &'a str> {
        let collection: Collection<Document> = db.collection("transactions");

        //filtering transactin hash
        let filter = doc! {"hash": self.hash.to_string()};
        let query = collection.find_one(filter).await;

        //check trx that it is in the transactions coll or not
        //if not then insertion will be done
        if let Ok(opt) = query {
            if opt.is_none() {
                let trx_to_doc = to_document(self).unwrap();
                match collection.insert_one(trx_to_doc).await {
                    Ok(_) => match collection.count_documents(doc! {}).await {
                        Ok(count) => {
                            if count > 1 && !leader.in_check {
                                match leader.timer {
                                    LeaderTime::On => {
                                        let now = Utc::now();

                                        if now > leader.time.unwrap() {
                                            Leader::start_voting(
                                                leader,
                                                db,
                                                connection_handler,
                                                swarm,
                                            )
                                            .await
                                        } else {
                                            Ok(())
                                        }
                                    }
                                    LeaderTime::Off => {
                                        leader.timer_start();
                                        Ok(())
                                    }
                                }
                            } else {
                                Ok(())
                            }
                        }
                        Err(_) => Err(
                            "Get count of transactions problem-(relay/practical/transaction 170)",
                        ),
                    },
                    Err(_) => {
                        Err("Transaction insertion problem-(relay/practical/transaction 173)")
                    }
                }
            } else {
                Ok(())
            }
        } else {
            Err("Querying transaction problem-(relay/practical/transaction 152)")
        }
    }
}
