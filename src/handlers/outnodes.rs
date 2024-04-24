use libp2p::{gossipsub::IdentTopic, PeerId, Swarm};

use super::{
    create_log::write_log,
    structures::{FullNodes, OutNode},
    CustomBehav
};

pub async fn handle_outnode(
    peerid: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    clients_topic: IdentTopic,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
    fullnodes: &mut Vec<FullNodes>,
    leader: &mut String
) {
    if let Some(index) = fullnodes.iter().position(|x| x.peer_id == peerid) {
        //remove validator if left the network
        fullnodes.remove(index);
    } else {
        //remove validator if its relay left the network
        for validator in fullnodes.clone() {
            if peerid == validator.relay {
                let index = fullnodes.iter().position(|f| peerid == f.relay);
                match index {
                    Some(i) => {
                        fullnodes.remove(i);
                    }
                    None => {}
                }
            }
        }
    }

    //remove next leader if there is no validator in the network
    if fullnodes.len() < 1 {
        leader.clear()
    }

    //say to network that a validator left from the network
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
}
