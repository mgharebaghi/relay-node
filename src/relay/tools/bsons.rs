use std::{fs::File, io::BufReader};

use mongodb::{
    bson::{doc, Document},
    Collection, Database,
};

use super::create_log::write_log;

pub struct Bson;

impl Bson {
    pub async fn add<'a>(
        db: &'a Database,
        collection_name: &str,
        bson: &str,
    ) -> Result<(), &'a str> {
        //open bson file that its address is in the bson argument
        let bson_addr = format!("/home/Downloads/etc/dump/Blockchain/{}", bson);
        let open_file = File::open(bson_addr);
        match open_file {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                let collection: Collection<Document> = db.collection(collection_name);
                collection.delete_many(doc! {}).await.unwrap();
                while let Ok(doc) = Document::from_reader(&mut reader) {
                    collection.insert_one(doc).await.unwrap();
                }
                Ok(write_log(&format!("{} Synced", collection_name)))
            }
            Err(_e) => Err("Your file address is incorrect!-(tools/bsons 31)"),
        }
    }
}
