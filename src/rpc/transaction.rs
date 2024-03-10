use std::sync::{Arc, Mutex};

use axum::{
    extract::{self},
    Extension, Json,
};
use futures::{channel::mpsc::Sender, SinkExt};
use libp2p::{gossipsub::IdentTopic, Swarm};
use mongodb::{
    bson::{to_document, Document},
    Collection,
};

use crate::{
    handlers::{check_trx, db_connection::blockchain_db, structures::Transaction}, write_log, CustomBehav
};

use super::server::TxRes;

pub async fn handle_transaction(
    // mut tx: Extension<Sender<String>>,
    extract::Json(transaction): extract::Json<Transaction>,
) -> Json<TxRes> {

    //insert transaction reciept into db
    let str_trx = serde_json::to_string(&transaction).unwrap();
    // match tx.send(str_trx.clone()).await {
    //     Ok(_) => {write_log("tx send works")}
    //     Err(e) => write_log(&format!("tx send problem: {}", e))
    // }
    write_log(&str_trx);
    check_trx::handle_transactions(str_trx).await;

    //send response to the client
    let tx_res = TxRes {
        hash: transaction.tx_hash,
        status: "sent".to_string(),
        description: "Wait for submit".to_string(),
    };
    return Json(tx_res);
}
