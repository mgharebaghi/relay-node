use libp2p::{futures::StreamExt, PeerId};
use mongodb::{
    bson::{doc, from_document, to_document, Document},
    Collection, Database,
};

use super::validator::Validator;

pub struct Waiting;

impl Waiting {
    pub async fn update<'a>(db: &'a Database, block_generator: &PeerId) -> Result<(), &'a str> {
        let collection: Collection<Document> = db.collection("validators");
        let query = collection.find(doc! {}).await;
        match query {
            Ok(mut cursor) => {
                let mut is_err = None;
                while let Some(result) = cursor.next().await {
                    match result {
                        Ok(doc) => {
                            let mut validator: Validator = from_document(doc.clone()).unwrap();
                            //if validator was generator of block its waiting should be count of validators in the network
                            //else its waiting should sets waiting - 1
                            if &validator.peerid == block_generator {
                                validator.waiting =
                                    collection.count_documents(doc! {}).await.unwrap() as u64;
                                let replacement = to_document(&validator).unwrap();
                                match collection.replace_one(doc, replacement).await {
                                    Ok(_) => {}
                                    Err(_) => {
                                        is_err.get_or_insert("Error during the replacing of document-(tools/waiting 31)");
                                        break;
                                    }
                                }
                            } else {
                                //if waiting is greater than 0 then it should sets waiting -1
                                if validator.waiting > 0 {
                                    validator.waiting -= 1;
                                    let replacement = to_document(&validator).unwrap();
                                    match collection.replace_one(doc, replacement).await {
                                        Ok(_) => {}
                                        Err(_) => {
                                            is_err.get_or_insert("Error during the replacing of document-(tools/waiting 41)");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            is_err.get_or_insert(
                                "Error in err arm of check result of cursor-(tools/waiting 49)",
                            );
                            break;
                        }
                    }
                }
                if is_err.is_none() {
                    Ok(())
                } else {
                    Err(is_err.unwrap())
                }
            }
            Err(_) => Err("Error during quering in-(tools/waiting 61)"),
        }
    }
}