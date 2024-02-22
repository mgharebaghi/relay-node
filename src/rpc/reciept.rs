use std::str::FromStr;

use axum::{extract, Json};
use libp2p::futures::StreamExt;
use mongodb::{
    bson::{doc, from_document, Document},
    Collection,
};
use rust_decimal::Decimal;

use crate::handlers::db_connection::blockchain_db;

use super::server::{RcptReq, RcptRes, Reciept, TxReq};

pub async fn handle_reciept(extract::Json(tx_req): extract::Json<TxReq>) -> Json<Reciept> {
    match blockchain_db().await {
        Ok(db) => {
            let reciept_coll: Collection<Document> = db.collection("reciept");
            let filter = doc! {"tx_hash": tx_req.tx_hash.clone()};
            let documnet = reciept_coll.find_one(filter, None).await.unwrap();
            match documnet {
                Some(doc) => {
                    let reciept: Reciept = from_document(doc).unwrap();
                    return Json(reciept);
                }
                None => {
                    let reciept = Reciept {
                        block_number: None,
                        hash: tx_req.tx_hash,
                        from: String::new(),
                        to: String::new(),
                        value: Decimal::from_str("0.0").unwrap(),
                        fee: Decimal::from_str("0.0").unwrap(),
                        status: "Error".to_string(),
                        description: "Transaction not found!".to_string(),
                        date: "".to_string(),
                    };
                    return Json(reciept);
                }
            }
        }
        Err(_) => {
            let reciept = Reciept {
                block_number: None,
                hash: tx_req.tx_hash,
                from: String::new(),
                to: String::new(),
                value: Decimal::from_str("0.0").unwrap(),
                fee: Decimal::from_str("0.0").unwrap(),
                status: "Error".to_string(),
                description: "Relay problem, try with anothers".to_string(),
                date: "".to_string(),
            };
            return Json(reciept);
        }
    }
}

pub async fn handle_user_reciepts(
    extract::Json(rcpt_req): extract::Json<RcptReq>,
) -> Json<RcptRes> {
    match blockchain_db().await {
        Ok(db) => {
            let reciept_coll: Collection<Document> = db.collection("reciept");
            let filter1 = doc! {"to": rcpt_req.public_key.clone()};
            let filter2 = doc! {"from": rcpt_req.public_key.clone()};
            let mut documents1 = reciept_coll.find(filter1, None).await.unwrap();
            let mut documents2 = reciept_coll.find(filter2, None).await.unwrap();
            let mut all_rcpts = Vec::new();
            while let Some(doc) = documents1.next().await {
                match doc {
                    Ok(doc) => {
                        let reciept: Reciept = from_document(doc).unwrap();
                        all_rcpts.push(reciept);
                    }
                    Err(_) => {}
                }
            }
            while let Some(doc) = documents2.next().await {
                match doc {
                    Ok(doc) => {
                        let reciept: Reciept = from_document(doc).unwrap();
                        all_rcpts.push(reciept);
                    }
                    Err(_) => {}
                }
            }
            let rcpt_res = RcptRes { all: all_rcpts, status: "done".to_string() };
            return Json(rcpt_res);
        }
        Err(_) => {
            let rcpt_res = RcptRes { all: Vec::new(), status: "Error! Relay problem, try with anothers.".to_string() };
            return Json(rcpt_res);
        }
    }
}
