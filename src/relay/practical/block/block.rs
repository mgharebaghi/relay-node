use mongodb::{
    bson::{doc, to_document, Document},
    Collection, Database,
};
use serde::{Deserialize, Serialize};
use sp_core::Pair;

use crate::relay::{
    practical::transaction::Transaction,
    tools::{utxo::UTXO, waiting::Waiting, HashMaker},
};

use super::{coinbase::Coinbase, header::Header};

// Define the structure of a block, including its header and body.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Block {
    pub header: Header,
    pub body: Body,
}

// Define the body of a block, which includes the coinbase transaction (encompassing rewards and fees)
// and other transactions that the block should have.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Body {
    pub coinbase: Coinbase,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub async fn validation<'a>(
        &self,
        last_block: &mut Vec<Self>,
        db: &'a Database,
    ) -> Result<&Self, &'a str> {
        // Check if the block is either the genesis block or if it correctly follows the last block
        if (last_block.len() > 0 && last_block[0].header.hash == self.header.previous)
            || self.header.previous == "This Is The Genesis Block".to_string()
        {
            // Check the block's signature to ensure its integrity
            let hash_data = serde_json::to_string(&self.body).unwrap();
            let hash = HashMaker::generate(&hash_data);
            let sign_check = sp_core::ed25519::Pair::verify(
                &self.header.signature.signatgure,
                hash,
                &self.header.signature.key,
            );

            // If the block's signature is valid, proceed to validate the transactions in the block's body
            if sign_check {
                let mut trx_err = None;
                let mut trx_backup: Vec<Transaction> = Vec::new();
                let trx_collection: Collection<Document> = db.collection("transactions");

                // Validate each transaction in the block's body
                for i in 0..self.body.transactions.len() {
                    let filter = doc! {"hash": self.body.transactions[i].hash.clone()};
                    let query = trx_collection.find_one(filter).await;
                    match query {
                        Ok(opt) => match opt {
                            None => {
                                match Transaction::validate(&self.body.transactions[i], db).await {
                                    Ok(transaction) => {
                                        match trx_collection
                                            .delete_one(doc! {"hash": &transaction.hash})
                                            .await
                                        {
                                            Ok(_) => {
                                                trx_backup.push(transaction.clone());
                                            }
                                            Err(_) => {}
                                        }
                                    }
                                    Err(e) => {
                                        trx_err.get_or_insert(e);
                                        break;
                                    }
                                }
                            }
                            Some(_) => {
                                match trx_collection
                                    .delete_one(doc! {"hash": &self.body.transactions[i].hash})
                                    .await
                                {
                                    Ok(_) => {
                                        trx_backup.push(self.body.transactions[i].clone());
                                    }
                                    Err(_) => {}
                                }
                            }
                        },
                        Err(_) => {
                            trx_err.get_or_insert(
                                "Querying transaction problem-(relay/practical/block 61)",
                            );
                        }
                    }
                }

                // If no transaction errors were found, validate the coinbase transaction and generate UTXOs
                if trx_err.is_none() {
                    match Coinbase::validation(
                        &self.body.coinbase,
                        last_block,
                        &self.body.transactions,
                    )
                    .await
                    {
                        Ok(_) => {
                            let mut utxo_err: Option<&str> = None;
                            // Generate new UTXOs for each unspent output in the coinbase transaction
                            for unspent in &self.body.coinbase.output.unspents {
                                match UTXO::generate(
                                    self.header.number,
                                    &self.body.coinbase.hash,
                                    &self.body.coinbase.output.hash,
                                    unspent,
                                    db,
                                )
                                .await
                                {
                                    Ok(_) => {}
                                    Err(e) => {
                                        utxo_err.get_or_insert(e);
                                        break;
                                    }
                                }
                            }

                            // Generate new UTXOs for each unspent output in the block's transactions
                            for trx in &self.body.transactions {
                                for unspent in &trx.output.unspents {
                                    match UTXO::generate(
                                        self.header.number,
                                        &trx.hash,
                                        &trx.output.hash,
                                        unspent,
                                        db,
                                    )
                                    .await
                                    {
                                        Ok(_) => {}
                                        Err(e) => {
                                            utxo_err.get_or_insert(e);
                                            break;
                                        }
                                    }
                                }
                            }

                            // If no UTXO generation errors occurred, update the waiting validators
                            if utxo_err.is_none() {
                                match Waiting::update(db, Some(&self.header.validator)).await {
                                    Ok(_) => Ok(self),
                                    Err(e) => {
                                        // If updating the waiting validators fails, restore the transactions from the backup
                                        for trx in trx_backup {
                                            let trx_doc = to_document(&trx).unwrap();
                                            match trx_collection.insert_one(trx_doc).await {
                                                Ok(_) => {}
                                                Err(_) => {}
                                            }
                                        }
                                        Err(e)
                                    }
                                }
                            } else {
                                // If UTXO generation fails, restore the transactions from the backup
                                for trx in trx_backup {
                                    let trx_doc = to_document(&trx).unwrap();
                                    match trx_collection.insert_one(trx_doc).await {
                                        Ok(_) => {}
                                        Err(_) => {}
                                    }
                                }
                                Err(utxo_err.unwrap())
                            }
                        }
                        Err(e) => {
                            // If coinbase validation fails, restore the transactions from the backup
                            for trx in trx_backup {
                                let trx_doc = to_document(&trx).unwrap();
                                match trx_collection.insert_one(trx_doc).await {
                                    Ok(_) => {}
                                    Err(_) => {}
                                }
                            }
                            Err(e)
                        }
                    }
                } else {
                    // If transaction validation fails, restore the transactions from the backup
                    for trx in trx_backup {
                        let trx_doc = to_document(&trx).unwrap();
                        match trx_collection.insert_one(trx_doc).await {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                    Err(trx_err.unwrap())
                }
            } else {
                Err("Block signature is wrong and Block rejected.")
            }
        } else {
            Err("Block validation problem!, previous hash doesn't match and Block rejected.")
        }
    }
}
