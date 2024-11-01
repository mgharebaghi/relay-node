use std::str::FromStr;

use axum::{
    extract::{self},
    Json,
};
use chrono::{SubsecRound, Utc};
use mongodb::{
    bson::{to_document, Document},
    Collection,
};
use rust_decimal::Decimal;

use crate::relay::practical::{db::Mongodb, transaction::Transaction};

use super::server::TxRes;

pub async fn handle_transaction(
    extract::Json(mut transaction): extract::Json<Transaction>,
) -> Json<TxRes> {
    let mut tx_res = TxRes {
        hash: transaction.hash.clone(),
        status: String::new(),
        description: String::new(),
    };

    match Mongodb::connect().await {
        Ok(db) => {
            let trx_coll: Collection<Document> = db.collection("transactions");
            transaction.date.clear();
            transaction.fee = transaction.value * Decimal::from_str("0.01").unwrap();
            transaction
                .date
                .push_str(&Utc::now().round_subsecs(0).to_string());
            let trx_doc = to_document(&transaction).unwrap();
            trx_coll.insert_one(trx_doc).await.unwrap();
            tx_res.status = "success".to_string();
        }
        Err(_) => {
            tx_res.status = "error".to_string();
            tx_res.description =
                "server has problem! please try with another provider.".to_string();
        } 
    }

    return Json(tx_res);
}
