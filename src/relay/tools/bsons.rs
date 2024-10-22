use std::{fs::File, io::BufReader};

use mongodb::{bson::Document, Collection, Database};

use super::create_log::write_log;

pub struct Bson;

impl Bson {
    pub async fn add<'a>(
        db: &'a Database,
        collection_name: &str,
        bson: &str,
    ) -> Result<(), &'a str> {
        // Construct the full path to the BSON file
        let bson_addr = format!("./etc/dump/Centichain/{}", bson);
        let open_file = File::open(bson_addr.clone());
        match open_file {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                // Get a reference to the MongoDB collection
                let collection: Collection<Document> = db.collection(collection_name);
                // Drop the existing collection if it exists
                db.collection::<Document>(collection_name).drop().await.ok();
                // Read and insert documents from the BSON file
                while let Ok(doc) = Document::from_reader(&mut reader) {
                    // Insert each document into the collection
                    collection.insert_one(doc).await.unwrap();
                }
                // Log successful synchronization
                Ok(write_log(&format!("{} Synced", collection_name)))
            }
            Err(_e) => Err("Your file address is incorrect!-(tools/bsons 31)"),
        }
    }
}
