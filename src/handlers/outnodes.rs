use libp2p::{PeerId, Swarm, gossipsub::IdentTopic};

use super::structures::{CustomBehav, OutNode};

pub fn handle_outnode(
    peerid: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    clients_topic: IdentTopic,
    client_topic_subscriber: &mut Vec<PeerId>,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
) {
    let outnode = OutNode { peer_id: peerid };
    let serialize_out_node = serde_json::to_string(&outnode).unwrap();
    if client_topic_subscriber.len() > 0 {
        swarm
            .behaviour_mut()
            .gossipsub
            .publish(clients_topic, serialize_out_node.as_bytes())
            .unwrap();
    }

    if relays.len() > 0 && clients.len() == 0 {
        swarm
            .behaviour_mut()
            .gossipsub
            .publish(relay_topic, "i dont have any clients".as_bytes())
            .unwrap();
    }
}