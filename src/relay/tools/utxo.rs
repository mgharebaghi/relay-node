use mongodb::{bson::{doc, from_document, to_document, Document}, Collection, Database};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use sp_core::ed25519::Public;

use crate::relay::practical::transaction::Unspent;

#[derive(Debug, Serialize, Deserialize)]
pub struct Person {
    pub wallet: Public,
    pub utxos: Vec<UTXO>,
}

impl Person {
    pub fn new(wallet: Public, utxos: Vec<UTXO>) -> Self {
        Self { wallet, utxos }
    }
}

// Define a UTXO model that includes the transaction hash, the unspent value,
// the output hash of its transaction, and the block number of the transaction
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct UTXO {
    pub block: u64,
    pub trx_hash: String,
    pub output_hash: String,
    pub unspent_hash: String,
    #[serde_as(as = "DisplayFromStr")]
    pub unspent: Decimal,
}

impl UTXO {
     //check a utxo that if exist remove it, else return error
     pub async fn check<'a>(&self, db: &Database, wallet: &Public) -> Result<(), &'a str> {
        //query from UTXOs collection for find person's utxos
        //if exist remove utxo else return error
        let collection: Collection<Document> = db.collection("UTXOs");
        let filter = doc! {"wallet": wallet.to_string()};
        let query = collection.find_one(filter).await;
        match query {
            Ok(opt) => match opt {
                Some(doc) => {
                    let mut person: Person = from_document(doc).unwrap(); //convert document to person structure
                    let index = person
                        .utxos
                        .iter()
                        .position(|u| u.unspent_hash == self.unspent_hash); //find index of utxo in the utxos variant of person structure that is a vector of utxo

                    //if there is an index remove it from person's utxos and update documnet of collection
                    //else return error
                    if let Some(i) = index {
                        let rm_utxo = person.utxos.remove(i); //put remove utxo from person to rm_utxo
                        let update = doc! {"$pull": {"utxos": rm_utxo.unspent_hash}}; //update command

                        //try to update the collection and if done return true
                        //else return error
                        match collection
                            .update_one(doc! {"wallet": wallet.to_string()}, update)
                            .await
                        {
                            Ok(_) => Ok(()),
                            Err(_) => Err("Error during the updating of utxos-(tools/utxo )"),
                        }
                    } else {
                        Err("UTXO does not exist!")
                    }
                }
                None => Err("UTXO does not exist!"),
            },
            Err(_) => Err("Problem from query of utxos collection-(tools/utxo 67)"),
        }
    }

     //insert outputs of a transactions as UTXO
     pub async fn generate<'a>(
        block: u64,
        trx_hash: &String,
        output_hash: &String,
        unspent: &Unspent,
        db: &Database,
    ) -> Result<(), &'a str> {
        let wallet = unspent.data.wallet;
        //make utxo with arguments for insert to database
        let utxo = Self {
            block: block,
            trx_hash: trx_hash.to_string(),
            output_hash: output_hash.to_string(),
            unspent_hash: unspent.hash.to_string(),
            unspent: unspent.data.value,
        };
        let collection: Collection<Document> = db.collection("UTXOs");
        let query = collection
            .find_one(doc! {"wallet": wallet.to_string()})
            .await;
        match query {
            Ok(opt) => {
                //if person was in the utxos database update it
                //else make and insert new person
                if let Some(doc) = opt {
                    let utxo_to_doc = to_document(&utxo).unwrap();
                    let update = doc! {"$push": {"utxos": utxo_to_doc}};
                    match collection.update_one(doc, update).await {
                        Ok(_) => Ok(()),
                        Err(_) => Err("Error while updating utxos-(tools/utxo 106)"),
                    }
                } else {
                    let new_person = Person::new(wallet, vec![utxo]);
                    let person_to_doc = to_document(&new_person).unwrap();
                    match collection.insert_one(person_to_doc).await {
                        Ok(_) => Ok(()),
                        Err(_) => Err("Error while inserting utxos-(tools/utxo 113)"),
                    }
                }
            }
            Err(_) => Err("Error during qury of mongodb-(tools/utxo 117)"),
        }
    }
}