use axum::{
    extract::{self},
    Json,
};
use mongodb::{
    bson::{to_document, Document},
    Collection,
};

use crate::handlers::{db_connection::blockchain_db, structures::Transaction};

use super::server::TxRes;

pub async fn handle_transaction(
    extract::Json(transaction): extract::Json<Transaction>,
) -> Json<TxRes> {
    let mut tx_res = TxRes {
        hash: transaction.tx_hash.clone(),
        status: String::new(),
        description: String::new(),
    };

    match blockchain_db().await {
        Ok(db) => {
            let trx_coll: Collection<Document> = db.collection("Transactions");
            let trx_doc = to_document(&transaction).unwrap();
            trx_coll.insert_one(trx_doc, None).await.unwrap();
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
