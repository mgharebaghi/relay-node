use libp2p::PeerId;
use mongodb::{
    bson::{doc, Document},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use super::waiting::Waiting;

#[derive(Debug, Serialize, Deserialize)]
pub struct WrongDoer {
    peerid: PeerId,
    cause: String,
}

impl WrongDoer {
    pub async fn remove<'a>(db: &'a Database, peerid: PeerId) -> Result<PeerId, &'a str> {
        let collection: Collection<Document> = db.collection("validators");
        let filter = doc! {"$or": [{"peerid": peerid.to_string()}, {"relay": peerid.to_string()}]}; // filter validators who their peerid is wrongdoer or its rela

        //get count of documents that will be removed for update validators' waiting
        match collection.count_documents(filter.clone()).await {
            //deleting validators
            Ok(mut count) => match collection.delete_many(filter).await {
                Ok(_) => {
                    let mut is_err = None;
                    while count > 0 {
                        //validators' waiting update for each deleted validator
                        match Waiting::update(db, Some(&peerid)).await {
                            Ok(_) => count -= 1,
                            Err(e) => {
                                is_err.get_or_insert(e);
                                break;
                            }
                        }
                    }

                    // if there is no any errors, return peerid
                    match is_err {
                        None => Ok(peerid),
                        Some(e) => Err(e),
                    }
                }
                Err(_) => Err("Deleting validators from mongodb has problem-(tools/wrongdoer 44)"),
            },
            Err(_) => Err("counting validators from mongodb has problem-(tools/wrongdoer 46)"),
        }
    }
}
