use libp2p::{gossipsub::TopicHash, PeerId, Swarm};

use super::structures::CustomBehav;

//send listener addresses to another relays and clients
pub fn send_address(
    topic: TopicHash,
    peer_id: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    my_addresses: Vec<String>,
    relay_topic_subscribers: &mut Vec<PeerId>,
    connections: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    client_topic_subscriber: &mut Vec<PeerId>,
    wallet_topic_subscriber: &mut Vec<PeerId>,

) {
    if topic.to_string() == "relay".to_string() {
        relay_topic_subscribers.push(peer_id);
        if connections.contains(&peer_id) && clients.len() > 0 {
            swarm
                .behaviour_mut()
                .gossipsub
                .publish(topic.clone(), "i have a client".as_bytes())
                .unwrap();
        }
        let my_addresses_str = serde_json::to_string(&my_addresses).unwrap();
        swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), my_addresses_str.as_bytes())
            .unwrap();
    }

    if topic.to_string() == "client".to_string() && connections.contains(&peer_id) {
        client_topic_subscriber.push(peer_id);
        let my_addresses_str = serde_json::to_string(&my_addresses).unwrap();
        swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), my_addresses_str.as_bytes())
            .unwrap();
    }

    if topic.to_string() == "tarnsaction" && connections.contains(&peer_id) {
        wallet_topic_subscriber.push(peer_id);
    }
}