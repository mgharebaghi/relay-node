use std::str::FromStr;

use async_std::stream::StreamExt;
use axum::{
    extract::{self, ws::WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    Json,
};
use mongodb::{
    bson::{doc, from_document, Document},
    Collection,
};
use rust_decimal::Decimal;

use crate::relay::{practical::db::Mongodb, tools::create_log::write_log};

use super::server::{RcptReq, RcptRes, Reciept, TxReq};

// Handle receipt lookup requests by transaction hash
pub async fn handle_reciept(extract::Json(tx_req): extract::Json<TxReq>) -> Json<Reciept> {
    match Mongodb::connect().await {
        Ok(db) => {
            // Query receipt collection for matching transaction hash
            let reciept_coll: Collection<Document> = db.collection("reciept");
            let filter = doc! {"hash": tx_req.tx_hash.clone()};
            let documnet = reciept_coll.find_one(filter).await.unwrap();
            match documnet {
                Some(doc) => {
                    // Return found receipt
                    let reciept: Reciept = from_document(doc).unwrap();
                    return Json(reciept);
                }
                None => {
                    // Return error receipt if transaction not found
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
            // Return error receipt if database connection fails
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

// Handle requests for all receipts associated with a public key
pub async fn handle_user_reciepts(
    extract::Json(rcpt_req): extract::Json<RcptReq>,
) -> Json<RcptRes> {
    match Mongodb::connect().await {
        Ok(db) => {
            // Query receipts where public key is sender or receiver
            let reciept_coll: Collection<Document> = db.collection("reciept");
            let filter1 = doc! {"to": rcpt_req.public_key.clone()};
            let filter2 = doc! {"from": rcpt_req.public_key.clone()};
            let mut documents1 = reciept_coll.find(filter1).await.unwrap();
            let mut documents2 = reciept_coll.find(filter2).await.unwrap();
            let mut all_rcpts = Vec::new();
            
            // Collect all receipts where user is receiver
            while let Some(doc) = documents1.next().await {
                match doc {
                    Ok(doc) => {
                        let reciept: Reciept = from_document(doc).unwrap();
                        all_rcpts.push(reciept);
                    }
                    Err(_) => {}
                }
            }
            
            // Collect all receipts where user is sender
            while let Some(doc) = documents2.next().await {
                match doc {
                    Ok(doc) => {
                        let reciept: Reciept = from_document(doc).unwrap();
                        all_rcpts.push(reciept);
                    }
                    Err(_) => {}
                }
            }
            
            // Return successful response with all receipts
            let rcpt_res = RcptRes {
                all: all_rcpts,
                status: "done".to_string(),
            };
            return Json(rcpt_res);
        }
        Err(_) => {
            // Return error if database connection fails
            let rcpt_res = RcptRes {
                all: Vec::new(),
                status: "Error! Relay problem, try with anothers.".to_string(),
            };
            return Json(rcpt_res);
        }
    }
}

// Initialize WebSocket connection for real-time receipt updates
pub async fn ws_reciept(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|socket| onchange(socket))
}

// Handle WebSocket connection and receipt change notifications
async fn onchange(mut socket: WebSocket) {
    match Mongodb::connect().await {
        Ok(db) => {
            let collection = db.collection::<Document>("reciepts");

            // Spawn a task to handle incoming messages
            tokio::spawn(async move {
                while let Some(msg) = socket.next().await {
                    if let Ok(msg) = msg {
                        match msg {
                            axum::extract::ws::Message::Text(hash) => {
                                // Create pipeline to watch for receipt updates
                                let pipeline = vec![doc! {
                                    "$match": {
                                        "operationType": "update",
                                        "fullDocument.hash": hash
                                    }
                                }];

                                // Configure change stream options
                                let options = mongodb::options::ChangeStreamOptions::builder()
                                    .full_document(mongodb::options::FullDocumentType::UpdateLookup)
                                    .build();

                                // Watch for changes and send updates to client
                                if let Ok(mut filtered_stream) = collection
                                    .watch()
                                    .pipeline(pipeline)
                                    .with_options(options)
                                    .await
                                {
                                    while let Some(change) = filtered_stream.next().await {
                                        if let Ok(change_doc) = change {
                                            let doc = change_doc.full_document.unwrap();
                                            if let Ok(json_str) = serde_json::to_string(&doc) {
                                                if socket
                                                    .send(axum::extract::ws::Message::Text(
                                                        json_str,
                                                    ))
                                                    .await
                                                    .is_err()
                                                {
                                                    write_log("Error sending message to client-(RPC-server/ reciept.rs/ 53)");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    write_log(
                                "Failed to create change stream-(RPC-server/ reciept.rs/ 61)",
                            );
                                }
                            }
                            axum::extract::ws::Message::Close(_) => {
                                // Client disconnected, break loop
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            });
        }
        Err(_) => {
            write_log("Error connecting to MongoDB-(RPC-server/ reciept.rs/ 106)");
        }
    }
}
