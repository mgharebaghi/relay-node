use axum::{
    extract,
    Json,
};

use mongodb::{bson::{doc, from_document, Document}, Collection};
use crate::handlers::{db_connection::blockchain_db, structures::Block};

use super::server::{BlockReq, BlockRes};

pub async fn handle_block(extract::Json(block_req): extract::Json<BlockReq>) -> Json<BlockRes> {
    match blockchain_db().await {
        Ok(db) => {
            let block_coll: Collection<Document> = db.collection("Blocks");
            let filter = doc! {"header.number": block_req.block_number.clone()};
            let documnet = block_coll.find_one(filter, None).await.unwrap();
            match documnet {
                Some(doc) => {
                    let block: Block = from_document(doc).unwrap();
                    return Json(BlockRes {
                        block: Some(block),
                        status: "".to_string(),
                    });
                }
                None => {
                    return Json(BlockRes {
                        block: None,
                        status: "Block not found!".to_string(),
                    });
                }
            }
        }
        Err(_) => {
            return Json(BlockRes {
                block: None,
                status: "Relay has problem! try with anothers.".to_string(),
            });
        }
    }
}
