use mongodb::{
    bson::{doc, to_document, Document},
    Collection, Database,
};

use super::{structures::FullNodes, structures::ImSync};

pub async fn handle_sync_message(str_msg: &String, db: Database) {
    if let Ok(new_sync_node) = serde_json::from_str::<ImSync>(&str_msg) {
        let outnode_coll: Collection<Document> = db.collection("outnodes");
        let validators_coll: Collection<Document> = db.collection("validators");
        let filter = doc! {"peerid": new_sync_node.peerid.to_string()};
        let cursor = outnode_coll.find_one(filter, None).await;
        if let Ok(opt) = cursor {
            if let None = opt {
                let doc = doc! {"peer_id": new_sync_node.peerid.to_string()};
                let vcursor = validators_coll.find_one(doc, None).await;
                if let Ok(option) = vcursor {
                    if let None = option {
                        let validators_count = validators_coll.count_documents(None, None).await.unwrap();
                        let new_fullnode = FullNodes {
                            relay: new_sync_node.relay,
                            peer_id: new_sync_node.peerid,
                            waiting: (validators_count + 1) as i64 * 2,
                            public_key: new_sync_node.public_key,
                        };
                        let validator_doc = to_document(&new_fullnode).unwrap();
                        validators_coll
                            .insert_one(validator_doc, None)
                            .await
                            .unwrap();
                    }
                }
            }
        }
    }
}
