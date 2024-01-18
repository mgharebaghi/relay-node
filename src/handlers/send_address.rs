use libp2p::{gossipsub::TopicHash, PeerId, Swarm};
use reqwest::Client;

use super::{create_log::write_log, structures::CustomBehav};

//send listener addresses to another relays and clients
pub async fn send_address(
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
            match swarm
                .behaviour_mut()
                .gossipsub
                .publish(topic.clone(), "i have a client".as_bytes())
            {
                Ok(_) => {}
                Err(e) => write_log(format!("{}", e)),
            }
        }
        let my_addresses_str = serde_json::to_string(&my_addresses).unwrap();
        match swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), my_addresses_str.as_bytes())
        {
            Ok(_) => {}
            Err(e) => write_log(format!("{}", e)),
        }
    }

    if topic.to_string() == "client".to_string() && connections.contains(&peer_id) {
        client_topic_subscriber.push(peer_id);
        let my_addresses_str = serde_json::to_string(&my_addresses).unwrap();
        match swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), my_addresses_str.as_bytes())
        {
            Ok(_) => {}
            Err(e) => write_log(format!("{}", e)),
        }

        if clients.len() >= 1 {
            let trim_my_addr = my_addresses[0].trim_start_matches("/ip4/");
            let my_ip = trim_my_addr.split("/").next().unwrap();
            let client = Client::new();
            let res = client
                .post("https://centichain.org/api/rpc")
                .body(my_ip.to_string())
                .send()
                .await;
            match res {
                Ok(_) => {}
                Err(_) => write_log(
                    "Can not send your public ip to the server in gossip messages check!"
                        .to_string(),
                ),
            }
        }
    }

    if topic.to_string() == "transaction" && connections.contains(&peer_id) {
        wallet_topic_subscriber.push(peer_id);
    }
}
