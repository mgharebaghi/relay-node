use libp2p::{gossipsub::IdentTopic, PeerId, Swarm};
use mongodb::{bson::{doc, Document}, Collection, Database};

use super::{
    create_log::write_log,
    structures::{FullNodes, OutNode},
    CustomBehav,
};

pub async fn handle_outnode(
    peerid: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    clients_topic: IdentTopic,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
    fullnodes: &mut Vec<FullNodes>,
    leader: &mut String,
    relay_topic_subscribers: &mut Vec<PeerId>,
    client_topic_subscriber: &mut Vec<PeerId>,
    im_first: &mut bool,
    dialed_addr: &mut Vec<String>,
    db: Database
) {
    //remove from clients topic if peerid is in the client topic subs
    if let Some(index) = client_topic_subscriber.iter().position(|c| c == &peerid) {
        write_log(&format!("connection closed with: {}", peerid));
        client_topic_subscriber.remove(index);
    }

    //remove from relay topic subscribers && remove from relays.dat file
    if let Some(index) = relay_topic_subscribers.iter().position(|id| id == &peerid) {
        relay_topic_subscribers.remove(index);
        if relay_topic_subscribers.len() == 0 {
            *im_first = true;
            write_log(&format!("Im first: {}", im_first));
        }
    }

    //remove peer from relays if it is in the relays
    if let Some(index) = relays.iter().position(|id| id == &peerid) {
        relays.remove(index);
    }

    //remove fullnode if closed connection pid is in the fullnodes
    if let Some(index) = fullnodes
        .iter()
        .position(|fullnode| fullnode.peer_id == peerid)
    {
        fullnodes.remove(index); //remove validator if left the network
        write_log("fullnode removed");
        //remove next leader if there is no validator in the network
        if fullnodes.len() < 1 {
            leader.clear();
            write_log("fullnode is 0 and leader is cleared");
        }
    }

    //remove fullnode if closed connection pid is in the fullnode relay pid
    if let Some(index) = fullnodes
        .iter()
        .position(|fullnode| fullnode.relay == peerid)
    {
        fullnodes.remove(index); //remove validator if left the network
        write_log("fullnode removed");
        //remove next leader if there is no validator in the network
        if fullnodes.len() < 1 {
            leader.clear();
            write_log("fullnode is 0 and leader is cleared");
        }
    }

    //remove left node from clients that have connection with current leader and synced
    if let Some(index) = clients.iter().position(|client| client == &peerid) {
        //say to network that a validator left from the network
        clients.remove(index);
        write_log("client removed");
    }

    //propagate left node to the network
    let outnode = OutNode { peer_id: peerid };
    let serialize_out_node = serde_json::to_string(&outnode).unwrap();
    match swarm
        .behaviour_mut()
        .gossipsub
        .publish(clients_topic, serialize_out_node.as_bytes())
    {
        Ok(_) => {}
        Err(_) => {}
    }

    //remove peer from dialed address if it is in the dialed addresses
    if let Some(index) = dialed_addr
        .iter()
        .position(|dialed| dialed.contains(&peerid.to_string()))
    {
        dialed_addr.remove(index);
    }

    //propagate don't have client to netwotk if clinets == 0
    if relays.len() > 0 && clients.len() == 0 {
        match swarm
            .behaviour_mut()
            .gossipsub
            .publish(relay_topic, "i dont have any clients".as_bytes())
        {
            Ok(_) => {}
            Err(e) => {
                write_log(&format!(
                    "gossipsub publish error in handle out node! line(61): {}",
                    e
                ));
            }
        }
    }

    //insert outnode into outnodes collection
    let outnode_coll:Collection<Document> = db.collection("outnodes");
    let doc= doc! {"peerid": peerid.to_string()};
    let cursor = outnode_coll.find_one(doc.clone(), None).await;
    if let Ok(opt) = cursor {
        if let None = opt {
            outnode_coll.insert_one(doc, None).await.unwrap();
        }
    }
}
