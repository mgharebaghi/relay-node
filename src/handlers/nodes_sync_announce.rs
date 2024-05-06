use mongodb::{
    bson::{doc, Document},
    Collection, Database,
};

use super::{structures::FullNodes, structures::ImSync};

pub async fn handle_sync_message(fullnodes: &mut Vec<FullNodes>, str_msg: &String, db: Database) {
    if let Ok(new_sync_node) = serde_json::from_str::<ImSync>(&str_msg) {
        let outnode_coll: Collection<Document> = db.collection("outnodes");
        let filter = doc! {"peerid": new_sync_node.peerid.to_string()};
        let cursor = outnode_coll.find_one(filter, None).await;
        if let Ok(opt) = cursor {
            if let None = opt {
                let new_fullnode = FullNodes {
                    relay: new_sync_node.relay,
                    peer_id: new_sync_node.peerid,
                    waiting: (fullnodes.len() + 1) as i64 * 2,
                    public_key: new_sync_node.public_key,
                };
                let mut fullnodes_pid = Vec::new();
                for i in fullnodes.clone() {
                    fullnodes_pid.push(i.peer_id.clone());
                }
                if !fullnodes_pid.contains(&new_fullnode.peer_id) {
                    fullnodes.push(new_fullnode);
                }
            }
        }
    }
}
