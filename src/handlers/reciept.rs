use std::str::FromStr;

use super::structures::{BlockHeader, CoinbaseTransaction, Reciept, Transaction};

use mongodb::{
    bson::{doc, to_document, Document},
    Collection, Database,
};
use rust_decimal::Decimal;

pub async fn insert_reciept(
    transaction: Transaction,
    block_number: Option<i64>,
    satatus: String,
    description: String,
    db: Database,
) {
    let reciept_coll: Collection<Document> = db.collection("reciept");
    let mut to = String::new();
    for output in transaction.output.output_data.utxos.clone() {
        if output.output_unspent.public_key
            != transaction.output.output_data.sigenr_public_keys[0].to_string()
        {
            to.push_str(&output.output_unspent.public_key);
        }
    }
    let reciept = Reciept {
        block_number,
        hash: transaction.tx_hash.clone(),
        from: transaction.output.output_data.sigenr_public_keys[0].to_string(),
        to,
        value: transaction.value,
        fee: transaction.fee,
        status: satatus.clone(),
        description,
        date: transaction.date,
    };

    let reciept_document = to_document(&reciept.clone()).unwrap();
    let filter = doc! {"hash": reciept.hash.clone()};
    let find_rec = reciept_coll.find_one(filter.clone()).await.unwrap();
    match find_rec {
        Some(_) => {
            reciept_coll
                .replace_one(filter, reciept_document)
                .await
                .unwrap();
        }
        None => {
            reciept_coll
                .insert_one(reciept_document)
                .await
                .unwrap();
        }
    }
}

pub async fn coinbase_reciept(
    transaction: CoinbaseTransaction,
    block_number: Option<i64>,
    satatus: String,
    description: String,
    block_header: BlockHeader,
    db: Database,
) {
    let reciept_coll: Collection<Document> = db.collection("reciept");
    let mut to = String::new();
    for output in transaction.output.utxos.clone() {
        to.clear();
        to.push_str(&output.output_unspent.public_key);
        let reciept = Reciept {
            block_number,
            hash: transaction.tx_hash.clone(),
            from: "Coinbase".to_string(),
            to: to.clone(),
            value: output.output_unspent.unspent,
            fee: Decimal::from_str("0.0").unwrap(),
            status: satatus.clone(),
            description: description.clone(),
            date: block_header.date.clone(),
        };

        let reciept_document = to_document(&reciept.clone()).unwrap();

        reciept_coll
            .insert_one(reciept_document)
            .await
            .unwrap();
    }
}
