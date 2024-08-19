use axum::{extract, Json};
use mongodb::{
    bson::{doc, from_document, Document},
    Collection,
};

use crate::handlers::tools::{db::Mongodb, utxo::Person};

use super::server::ReqForUtxo;

pub async fn handle_utxo(extract::Json(utxo_req): extract::Json<ReqForUtxo>) -> Json<Person> {
    match Mongodb::connect().await {
        Ok(db) => {
            let utxo_coll: Collection<Document> = db.collection("UTXOs");
            let filter = doc! {"public_key": utxo_req.public_key.clone()};
            let documnet = utxo_coll.find_one(filter).await.unwrap();
            match documnet {
                Some(doc) => {
                    let person: Person = from_document(doc).unwrap();
                    return Json(person);
                }
                None => {
                    let person = Person::new(utxo_req.public_key.parse().unwrap(), Vec::new());
                    return Json(person);
                }
            }
        }
        Err(_) => {
            let person = Person::new(utxo_req.public_key.parse().unwrap(), Vec::new());
            return Json(person);
        }
    }
}
