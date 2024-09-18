use mongodb::{
    bson::{doc, from_document, Document},
    options::FindOneOptions,
    Collection, Database,
};

use crate::relay::practical::block::block::Block;

pub struct LastBlock;

impl LastBlock {
    pub async fn get<'a>(db: &'a Database) -> Result<Option<Block>, &'a str> {
        let collection: Collection<Document> = db.collection("Blocks");
        let sort = doc! {"header.number": -1};
        let option = FindOneOptions::builder().sort(sort).build();
        let query = collection.find_one(doc! {}).with_options(option).await;

        match query {
            Ok(opt) => match opt {
                Some(doc) => {
                    let block: Block = from_document(doc).unwrap();
                    Ok(Some(block))
                }
                None => Ok(None),
            },
            Err(_) => Err("Problem during get last block-(relay/tools/get_last_block 16)"),
        }
    }
}
