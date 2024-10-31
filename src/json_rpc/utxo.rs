// Import required dependencies for WebSocket, JSON handling, and MongoDB
use axum::{
    extract::{self, ws::WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    Json,
};
use futures::StreamExt;
use mongodb::{
    bson::{doc, from_document, Document},
    Collection,
};
use rust_decimal::Decimal;

// Import custom modules for database and UTXO handling
use crate::relay::{practical::db::Mongodb, tools::{create_log::write_log, utxo::Person}};

use super::server::ReqForUtxo;

// Handle requests to get UTXO information for a given public key
pub async fn handle_utxo(extract::Json(utxo_req): extract::Json<ReqForUtxo>) -> Json<Person> {
    match Mongodb::connect().await {
        Ok(db) => {
            // Query UTXOs collection for the given public key
            let utxo_coll: Collection<Document> = db.collection("UTXOs");
            let filter = doc! {"public_key": utxo_req.public_key.clone()};
            let documnet = utxo_coll.find_one(filter).await.unwrap();
            match documnet {
                Some(doc) => {
                    // Return existing UTXO data if found
                    let person: Person = from_document(doc).unwrap();
                    return Json(person);
                }
                None => {
                    // Create new empty UTXO record if not found
                    let person = Person::new(utxo_req.public_key.parse().unwrap(), Vec::new());
                    return Json(person);
                }
            }
        }
        Err(_) => {
            // Return empty UTXO record on database error
            let person = Person::new(utxo_req.public_key.parse().unwrap(), Vec::new());
            return Json(person);
        }
    }
}

// Initialize WebSocket connection for real-time UTXO updates
pub async fn handle_utxo_ws(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|socket| onchange(socket))
}

// Handle WebSocket connection and UTXO change notifications
async fn onchange(mut socket: WebSocket) {
    match Mongodb::connect().await {
        Ok(db) => {
            let collection = db.collection::<Document>("UTXOs");

            // Spawn a task to handle incoming messages
            tokio::spawn(async move {
                while let Some(msg) = socket.next().await {
                    if let Ok(msg) = msg {
                        match msg {
                            axum::extract::ws::Message::Text(wallet) => {
                                // Create pipeline to watch for UTXO updates
                                let pipeline = vec![doc! {
                                    "$match": {
                                        "operationType": "update",
                                        "fullDocument.wallet": wallet
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
                                            // Calculate and send total UTXO balance
                                            let doc = change_doc.full_document.unwrap();
                                            let person: Person = from_document(doc).unwrap();
                                            let sum_utxos: Decimal =
                                                person.utxos.iter().map(|utxo| utxo.unspent).sum();
                                            if socket
                                                .send(axum::extract::ws::Message::Text(
                                                    sum_utxos.to_string(),
                                                ))
                                                .await
                                                .is_err()
                                            {
                                                write_log("Error sending message to client-(RPC-server/ reciept.rs/ 53)");
                                                break;
                                            } else {
                                                println!("utxo sent: {}", sum_utxos.to_string());
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
                                // Handle client disconnection
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            });
        }
        Err(_) => {
            write_log("Error connecting to MongoDB-(RPC-server/ utxo.rs/ 43)");
        }
    }
}
