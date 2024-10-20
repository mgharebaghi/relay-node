use async_std::stream::StreamExt;
use async_stream::stream;
use axum::response::sse::{Event, Sse};
use futures::stream::BoxStream;
use futures::stream::Stream;
use mongodb::{
    bson::{doc, from_document, Document},
    Collection,
};
use std::convert::Infallible;
use std::pin::Pin;

use crate::relay::{
    practical::{block::block::Block, db::Mongodb},
    tools::create_log::write_log,
};

// Handler for SSE
pub async fn centis_sse_handler(
) -> Sse<Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>>> {
    Sse::new(Box::pin(realtime_centis()))
}

pub fn realtime_centis() -> BoxStream<'static, Result<Event, Infallible>> {
    Box::pin(stream! {
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
                            // Send the coinbase reward as an SSE event
                            yield Ok(Event::default().data(block.body.coinbase.reward.to_string()));
                        }
                    }
                    Err(e) => {
                        write_log(&format!("Error in watching of mongodb: {:?}", e));
                        yield Ok(Event::default().data("Error occurred while watching MongoDB"));
                    }
                }
            }
            Err(e) => {
                write_log(&format!("Error in connecting to mongodb: {:?}", e));
                yield Ok(Event::default().data("Error occurred while connecting to MongoDB"));
            }
        }
    })
}
