use libp2p::{gossipsub::IdentTopic, PeerId, Swarm};

use super::{
    create_log::write_log,
    structures::{CustomBehav, FullNodes, OutNode},
};

pub async fn handle_outnode(
    peerid: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    clients_topic: IdentTopic,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
    fullnodes: &mut Vec<FullNodes>,
) {
    if let Some(index) = fullnodes.iter().position(|x| x.peer_id == peerid) {
        println!("remove fullnode");
        fullnodes.remove(index);
    } else {
        for validator in fullnodes.clone() {
            if peerid == validator.relay {
                let index = fullnodes.iter().position(|f| peerid == f.relay);
                match index {
                    Some(i) => {
                        println!("remove fullnode when it has relay that is outed");
                        fullnodes.remove(i);
                    }
                    None => {}
                }
            }
        }
    }

    let outnode = OutNode { peer_id: peerid };
    let serialize_out_node = serde_json::to_string(&outnode).unwrap();
    match swarm
        .behaviour_mut()
        .gossipsub
        .publish(clients_topic, serialize_out_node.as_bytes())
    {
        Ok(_) => {}
        Err(e) => {
            println!("gossip outnode problem:\n{}", e);
            write_log("gossipsub publish error in handle out node(client_topic)!".to_string());
        }
    }

    if relays.len() > 0 && clients.len() == 0 {
        match swarm
            .behaviour_mut()
            .gossipsub
            .publish(relay_topic, "i dont have any clients".as_bytes())
        {
            Ok(_) => {}
            Err(_) => {
                write_log("gossipsub publish error in handle out node(relay_topic)!".to_string());
            }
        }
    }
}
