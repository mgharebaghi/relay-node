use async_std::stream::StreamExt;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use mongodb::{
    bson::{doc, from_document, Document},
    Collection,
};

use crate::relay::{
    practical::{block::block::Block, db::Mongodb},
    tools::create_log::write_log,
};

// Handler for WebSocket
pub async fn centis_ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    // ... existing code ...
    match Mongodb::connect().await {
        Ok(db) => {
            let block_coll: Collection<Document> = db.collection("Blocks");
            let pipeline = vec![doc! { "$match": { "operationType": "insert" }}];

            match block_coll.watch().pipeline(pipeline).await {
                Ok(mut watcher) => {
                    while let Some(Ok(change)) = watcher.next().await {
                        let document: Document = change.full_document.unwrap();
                        let block: Block = from_document(document).unwrap();
                        
                        // Send the coinbase reward as a WebSocket message
                        if let Err(e) = socket.send(Message::Text(block.body.coinbase.reward.to_string())).await {
                            write_log(&format!("Error sending WebSocket message: {:?}", e));
                            break;
                        }
                    }
                }
                Err(e) => {
                    write_log(&format!("Error in watching MongoDB: {:?}", e));
                    let _ = socket.send(Message::Text("Error occurred while watching MongoDB".to_string())).await;
                }
            }
        }
        Err(e) => {
            write_log(&format!("Error in connecting to MongoDB: {:?}", e));
            let _ = socket.send(Message::Text("Error occurred while connecting to MongoDB".to_string())).await;
        }
    }
    // ... existing code ...
}
