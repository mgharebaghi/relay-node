use mongodb::{
    bson::{doc, to_document, Document},
    Collection, Database,
};

use super::{structures::Validator, structures::VSync};

pub async fn handle_sync_message(str_msg: &String, db: Database) {
    if let Ok(new_sync_node) = serde_json::from_str::<VSync>(&str_msg) {
        let outnode_coll: Collection<Document> = db.collection("outnodes");
        let validators_coll: Collection<Document> = db.collection("validators");
        let filter = doc! {"peerid": new_sync_node.peerid.to_string()};
        let cursor = outnode_coll.find_one(filter).await;
        if let Ok(opt) = cursor {
            if let None = opt {
                let doc = doc! {"peer_id": new_sync_node.peerid.to_string()};
                let vcursor = validators_coll.find_one(doc).await;
                if let Ok(option) = vcursor {
                    if let None = option {
                        let validators_count =
                            validators_coll.count_documents(doc! {}).await.unwrap();
                        let waiting = if validators_count > 0 {
                            (validators_count * 2) as u64
                        } else {
                            0
                        };
                        let new_fullnode = Validator {
                            peerid: new_sync_node.peerid,
                            relay: new_sync_node.relay,
                            wallet: new_sync_node.public_key,
                            waiting,
                        };
                        let validator_doc = to_document(&new_fullnode).unwrap();
                        validators_coll.insert_one(validator_doc).await.unwrap();
                    }
                }
            }
        }
    }
}
