use libp2p::{gossipsub::TopicHash, PeerId, Swarm};

use super::{create_log::write_log, structures::CustomBehav};

//send listener addresses to another relays and clients
pub fn send_address(
    topic: TopicHash,
    peer_id: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    relay_topic_subscribers: &mut Vec<PeerId>,
    connections: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    client_topic_subscriber: &mut Vec<PeerId>,
) {
    if topic.to_string() == "relay".to_string() {
        relay_topic_subscribers.push(peer_id);
        if connections.contains(&peer_id) && clients.len() > 0 {
            match swarm
                .behaviour_mut()
                .gossipsub
                .publish(topic.clone(), "i have a client".as_bytes())
            {
                Ok(_) => {}
                Err(e) => write_log(&format!("{}", e)),
            }
        }
    }

    if topic.to_string() == "client".to_string() && connections.contains(&peer_id) {
        client_topic_subscriber.push(peer_id);
    }

    if topic.to_string() == "sse".to_string() {
        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
    }
}
