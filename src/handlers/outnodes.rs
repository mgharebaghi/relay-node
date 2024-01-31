use libp2p::{gossipsub::IdentTopic, PeerId, Swarm};
use reqwest::Client;

use super::{
    create_log::write_log,
    structures::{CustomBehav, FullNodes, OutNode},
};

pub async fn handle_outnode(
    peerid: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    clients_topic: IdentTopic,
    client_topic_subscriber: &mut Vec<PeerId>,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
    my_addresses: &mut Vec<String>,
    fullnodes: &mut Vec<FullNodes>
) {
    if let Some(index) = fullnodes.iter().position(|x| x.peer_id == peerid) {
        fullnodes.remove(index);
    }
    let outnode = OutNode { peer_id: peerid };
    let serialize_out_node = serde_json::to_string(&outnode).unwrap();
    if client_topic_subscriber.len() > 0 {
        match swarm
            .behaviour_mut()
            .gossipsub
            .publish(clients_topic, serialize_out_node.as_bytes())
        {
            Ok(_) => {}
            Err(_) => {
                write_log("gossipsub publish error in handle out node(client_topic)!".to_string());
            }
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

    if clients.len() < 1 {
        let client = Client::new();
        let trim_my_addr = my_addresses[0].trim_start_matches("/ip4/");
        let my_ip = trim_my_addr.split("/").next().unwrap();
        match client
            .post("https://centichain.org/api/rmrpc")
            .body(my_ip.to_string())
            .send()
            .await
        {
            Ok(_) => {}
            Err(_) => {
                write_log("can not post the ip for remove rpc in outnodes section!".to_string());
            }
        }
    }
}
