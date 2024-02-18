use axum::{extract, Json};
use mongodb::{
    bson::{doc, from_document, Document},
    Collection,
};

use crate::handlers::{
    db_connection::blockchain_db,
    structures::UTXO,
};

use super::server::ReqForUtxo;

pub async fn handle_utxo(extract::Json(utxo_req): extract::Json<ReqForUtxo>) -> Json<UTXO> {
    match blockchain_db().await {
        Ok(db) => {
            let utxo_coll: Collection<Document> = db.collection("UTXOs");
            let filter = doc! {"public_key": utxo_req.public_key.clone()};
            let documnet = utxo_coll.find_one(filter, None).await.unwrap();
            match documnet {
                Some(doc) => {
                    let utxo: UTXO = from_document(doc).unwrap();
                    return Json(utxo);
                }
                None => {
                    let utxo = UTXO {
                        public_key: utxo_req.public_key,
                        utxos: Vec::new(),
                    };
                    return Json(utxo);
                }
            }
        }
        Err(_) => {
            let utxo = UTXO {
                public_key: "".to_string(),
                utxos: Vec::new(),
            };
            return Json(utxo);
        }
    }
}
