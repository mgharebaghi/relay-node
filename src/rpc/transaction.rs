use axum::{
    extract::{self},
    Json,
};

use crate::{
    handlers::{check_trx, structures::Transaction},
    propagate_trx,
};

use super::server::TxRes;

pub async fn handle_transaction(
    // mut tx: Extension<Sender<String>>,
    extract::Json(transaction): extract::Json<Transaction>,
) -> Json<TxRes> {
    //insert transaction reciept into db
    let str_trx = serde_json::to_string(&transaction).unwrap();
    propagate_trx(str_trx.clone());
    check_trx::handle_transactions(str_trx).await;

    //send response to the client
    let tx_res = TxRes {
        hash: transaction.tx_hash,
        status: "sent".to_string(),
        description: "Wait for submit".to_string(),
    };
    return Json(tx_res);
}
