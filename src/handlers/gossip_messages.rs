use std::{fs::OpenOptions, io::{BufWriter, Write}};

use libp2p::{
    gossipsub::{IdentTopic, Message}, Multiaddr, PeerId, Swarm
};

use crate::handlers::structures::{ImSync, OutNode};

use super::{create_log::write_log, get_addresses::get_addresses, handle_messages::msg_check, structures::{CustomBehav, FullNodes, NextLeader}};

pub async fn handle_gossip_message(
    propagation_source: PeerId,
    local_peer_id: PeerId,
    message: Message,
    clients: &mut Vec<PeerId>,
    relays: &mut Vec<PeerId>,
    swarm: &mut Swarm<CustomBehav>,
    relay_topic: IdentTopic,
    connections: &mut Vec<PeerId>,
    relay_topic_subscribers: &mut Vec<PeerId>,
    my_addresses: &mut Vec<String>,
    leader: &mut String, 
    fullnodes: &mut Vec<FullNodes>,
) {
    
    msg_check(message.clone(), leader, fullnodes, relays, propagation_source, swarm, connections, local_peer_id).await;

    match String::from_utf8(message.data.clone()) {
        Ok(msg) => {

            //handle next leader msg
            let mut fpids = Vec::new();
            for fullnode in fullnodes.clone() {
                fpids.push(fullnode.peer_id);
            }
            if let Ok(identifier) = serde_json::from_str::<NextLeader>(&msg) {
                if leader.len() > 0 && fpids.contains(&identifier.identifier_peer_id) && fpids.contains(&identifier.next_leader) {
                    leader.clear();
                    leader.push_str(&identifier.next_leader.to_string());
                } else {
                    write_log("identifier is not true! recieved block (line 96)".to_string())
                }
            }
            //get new realay addresses and add it to relays file
            if let Ok(addresses) = serde_json::from_str::<Vec<String>>(&msg) {
                get_addresses(addresses, local_peer_id, my_addresses);
            }

            //Relay announcement
            if let Ok(new_sync_node) = serde_json::from_str::<ImSync>(&msg) {
                if !relay_topic_subscribers.contains(&propagation_source) {
                    let new_sync_node_pid = new_sync_node.peerid;
                    let mut count_exist = 0;
                    for i in clients.clone() {
                        if i == new_sync_node_pid {
                            count_exist += 1;
                        }
                    }
                    if count_exist == 0 {
                        clients.push(new_sync_node_pid);
                    }
                    if relay_topic_subscribers.len() > 0 && clients.len() == 1 {
                        match swarm
                            .behaviour_mut()
                            .gossipsub
                            .publish(relay_topic, "i have a client".as_bytes()) {
                                Ok(_) => {
                          
                                }
                                Err(_) => write_log("gossipsub publish problem in gossip_messasged(relay announcement)!".to_string())
                            }
                    }
                }
            }

            if msg == "i have a client".to_string() {
                if connections.contains(&propagation_source) && !relays.contains(&propagation_source) {
                    relays.push(propagation_source);
                }
            }

            if msg == "i dont have any clients".to_string() && relays.contains(&propagation_source)
            {
                let index_option = relays.iter().position(|relay| *relay == propagation_source);
                match index_option {
                    Some(index) => {
                        relays.remove(index);
                    }
                    None => {}
                }
            }

            //get relay full address and insert it to relays file
            if let Ok(relayaddr) = serde_json::from_str::<Multiaddr>(&msg) {
                let relays_file = OpenOptions::new().append(true).write(true).open("/etc/relays.dat");
                match relays_file {
                    Ok(file) => {
                        let mut writer = BufWriter::new(file);
                        writeln!(writer, "{}", relayaddr.to_string()).unwrap();
                    }
                    Err(_) => {}
                }
            }

            //handle left nodes
            if let Ok(outnode) = serde_json::from_str::<OutNode>(&msg) {
                if let Some(index) = fullnodes
                .iter()
                .position(|x| x.peer_id == outnode.peer_id)
                    {
                        fullnodes.remove(index);
                    } else {
                        for fullnode in fullnodes.clone() {
                            if outnode.peer_id == fullnode.relay {
                                let index = fullnodes.iter().position(|x| x.relay == outnode.peer_id);
                                match index {
                                    Some(i) => {
                                        fullnodes.remove(i);
                                    }
                                    None => {}
                                }
                            }
                        }
                    }
            }

        }
        Err(_) => write_log("convert gossip message to string problem!".to_string()),
    } //convert messages to string
}
