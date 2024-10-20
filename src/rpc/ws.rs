use async_std::stream::StreamExt;
use axum::{extract::{ws::{Message, WebSocket}, WebSocketUpgrade}, response::IntoResponse};
use mongodb::{bson::{doc, from_document, Document}, Collection};

use crate::relay::{practical::{block::block::Block, db::Mongodb}, tools::create_log::write_log};

// Handler for WebSocket upgrade
pub async fn centis_socket_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(realtime_centis)
}

async fn realtime_centis(mut socket: WebSocket) {
    match Mongodb::connect().await {
        Ok(db) => {
            // Get the "Blocks" collection from MongoDB
            let block_coll: Collection<Document> = db.collection("Blocks");
            // Create a pipeline to watch for insert operations
            let pipeline = vec![doc! { "$match": {
                "operationType": "insert"
            }}];

            match block_coll.watch().pipeline(pipeline).await {
                Ok(mut watcher) => {
                    // Continuously watch for new blocks
                    while let Some(Ok(change)) = watcher.next().await {
                        // Extract the full document from the change stream
                        let document: Document = change.full_document.unwrap();
                        // Convert the document to a Block struct
                        let block: Block = from_document(document).unwrap();
                        // Send the coinbase reward to the WebSocket client
                        match socket.send(Message::Text(block.body.coinbase.reward.to_string())).await {
                            Ok(_) => (
                                
                            ),
                            Err(e) => {
                                write_log(&format!("Error in sending block-line(24): {:?}", e));
                            }
                        }
                    }
                }
                Err(e) => {
                    write_log(&format!("Error in watching of mongodb-line(138): {:?}", e));
                }
            }
        }
        Err(e) => {
            write_log(&format!("Error in connecting to mongodb-line(37): {:?}", e));
        }
    }
}
