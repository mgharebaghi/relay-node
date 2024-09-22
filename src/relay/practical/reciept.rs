use chrono::{SubsecRound, Utc};
use mongodb::{
    bson::{doc, to_document, Document},
    Collection, Database,
};
use rust_decimal::Decimal;
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};

use super::{block::coinbase::Coinbase, transaction::Transaction};

#[serde_as]
#[derive(Debug, Serialize)]
pub struct Reciept {
    block: Option<u64>,
    hash: String,
    from: String,
    to: String,
    #[serde_as(as = "DisplayFromStr")]
    value: Decimal,
    status: String,
    description: String,
    date: String,
}

impl Reciept {
    fn new(
        block: Option<u64>,
        hash: String,
        from: String,
        to: String,
        value: Decimal,
        status: String,
        description: String,
        date: String,
    ) -> Self {
        Self {
            block,
            hash,
            from,
            to,
            value,
            status,
            description,
            date,
        }
    }

    pub async fn insertion<'a>(
        block: Option<u64>,
        transaction: Option<&Transaction>,
        coinbase: Option<&Coinbase>,
        db: &'a Database,
    ) -> Result<(), &'a str> {
        let collection: Collection<Document> = db.collection("reciepts");

        //define hash. if coinbase was some hashe will be coinbase hash
        //else hash will be transaction hash
        let hash = if coinbase.is_some() {
            coinbase.unwrap().hash.clone()
        } else {
            transaction.unwrap().hash.clone()
        };

        //define from. if coinbase was some from will be Coinbase
        //else from will be transaction from
        let from = if coinbase.is_some() {
            "Coinbase".to_string()
        } else {
            transaction.unwrap().signature[0].key.to_string()
        };

        //defin to
        let mut to = String::new();

        //is_err is for get errors while make reciepts
        let mut is_err = None;

        //if coinbase was some then to will be coinbases unspents wallets
        //else to will be transaction output if output wallet doesn't be address of from
        if coinbase.is_some() {
            for unspent in coinbase.unwrap().output.unspents.clone() {
                to.clear();
                to.push_str(&unspent.data.wallet.to_string().clone());

                let reciept = Self::new(
                    block,
                    hash.clone(),
                    from.clone(),
                    to.clone(),
                    unspent.data.value,
                    "Confirmed".to_string(),
                    "This is coinbase  rewarding".to_string(),
                    Utc::now().round_subsecs(0).to_string(),
                );

                let doc = to_document(&reciept).unwrap();
                match collection.insert_one(doc).await {
                    Ok(_) => {}
                    Err(_) => {
                        is_err.get_or_insert(
                            "Error while insering reciept-(relay/practical/reciept 102)",
                        );
                    }
                }
            }
        } else {
            to.clear();

            for unspent in transaction.unwrap().output.unspents.clone() {
                if unspent.data.wallet.to_string() != from {
                    to.push_str(&unspent.data.wallet.to_string().clone());
                    break;
                }
            }

            let reciept = Self::new(
                block,
                hash.clone(),
                from.clone(),
                to.clone(),
                transaction.unwrap().value,
                "Pending".to_string(),
                "Waiting for confirmation".to_string(),
                Utc::now().round_subsecs(0).to_string(),
            );

            let doc = to_document(&reciept).unwrap();
            match collection.insert_one(doc).await {
                Ok(_) => {}
                Err(_) => {
                    is_err.get_or_insert(
                        "Error while insering reciept-(relay/practical/reciept 133)",
                    );
                }
            }
        }

        //return ok if is err is none
        //else return error
        match is_err {
            None => Ok(()),
            Some(e) => Err(e),
        }
    }

    //confirmation will use in block validation and if transactions was ok reciept will be comfirmed by confirmation
    pub async fn confirmation<'a>(
        db: &'a Database,
        hash: &String,
        block: &u64,
    ) -> Result<(), &'a str> {
        let collection: Collection<Document> = db.collection("reciepts");
        let filter = doc! {"hash": hash};
        let update = doc! {"$set": {"status": "Confirmed".to_string(), "description": "It was confirmed and placed in a block".to_string(), "block": block.to_string()}};

        match collection.update_one(filter, update).await {
            Ok(_) => Ok(()),
            Err(_) => Err("Error while updating reciept-(relay/practical/reciept 159)"),
        }
    }
}
