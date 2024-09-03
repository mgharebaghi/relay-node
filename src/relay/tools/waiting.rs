use libp2p::{futures::StreamExt, PeerId};
use mongodb::{
    bson::{doc, from_document, to_document, Document},
    Collection, Database,
};

use crate::relay::practical::validator::Validator;

pub struct Waiting;

impl Waiting {
    pub async fn update<'a>(
        db: &'a Database,
        block_generator: Option<&PeerId>,
    ) -> Result<(), &'a str> {
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
                            if block_generator.is_some()
                                && &validator.peerid == block_generator.unwrap()
                            {
                                validator.waiting =
                                    collection.count_documents(doc! {}).await.unwrap() as u64;
                                let replacement = to_document(&validator).unwrap();
                                match collection.replace_one(doc, replacement).await {
                                    Ok(_) => {}
                                    Err(_) => {
                                        is_err.get_or_insert("Error during the replacing of document-(tools/waiting 36)");
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
                                            is_err.get_or_insert("Error during the replacing of document-(tools/waiting 48)");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            is_err.get_or_insert(
                                "Error in err arm of check result of cursor-(tools/waiting 57)",
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
            Err(_) => Err("Error during quering in-(tools/waiting 69)"),
        }
    }

    //return new waiting as number for set it to new validator that gossips itself with vsync message
    pub async fn new<'a>(db: &'a Database) -> Result<u64, &'a str> {
        //check count of vadator documents and then return it if it was bigger than 0
        let collection: Collection<Document> = db.collection("validators");
        let coun_query = collection.count_documents(doc! {}).await;
        match coun_query {
            Ok(count) => {
                if count > 0 {
                    Ok(count * 2)
                } else {
                    Ok(0)
                }
            }
            Err(_) => Err("Error while get count of validators-(relay/tools/waiting 84)"),
        }
    }
}
