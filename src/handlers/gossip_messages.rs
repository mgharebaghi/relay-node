use std::{fs::OpenOptions, io::{BufWriter, Write}};

use libp2p::{
    gossipsub::{IdentTopic, Message}, Multiaddr, PeerId, Swarm
};
use mongodb::{bson::{doc, Document}, Collection, Database};

use crate::handlers::structures::{VSync, OutNode};

use super::{create_log::write_log, get_addresses::get_addresses, handle_messages::msg_check, structures::NextLeader, CustomBehav};

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
    db: Database
) {
    msg_check(message.clone(), leader, relays, propagation_source, swarm, connections, local_peer_id, db.clone()).await;

    match String::from_utf8(message.data.clone()) {
        Ok(msg) => {
            let validators_coll:Collection<Document> = db.collection("validators");
            //handle next leader msg
            if let Ok(identifier) = serde_json::from_str::<NextLeader>(&msg) {
                let filter1 = doc! {"peer_id": identifier.identifier_peer_id.to_string()};
                let filter2 = doc! {"peer_id": identifier.next_leader.to_string()};
                let find1 = validators_coll.find_one(filter1).await;
                if let Ok(opt) = find1 {
                    if let Some(_) = opt {
                        let find2 = validators_coll.find_one(filter2).await;
                        if let Ok(option) = find2 {
                            if let Some(_) = option  {
                                //remove validators from left leader
                                let leader_query = doc! {"peer_id": leader.clone()};
                                validators_coll.delete_one(leader_query).await.unwrap();

                                //set new leader that recieved from tru identifier
                                leader.clear();
                                leader.push_str(&identifier.next_leader.to_string());
                            }
                        }
                    }
                }
            }

            //get new realay addresses and add it to relays file
            if let Ok(addresses) = serde_json::from_str::<Vec<String>>(&msg) {
                get_addresses(addresses, local_peer_id, my_addresses);
            }

            //Relay announcement
            if let Ok(new_sync_node) = serde_json::from_str::<VSync>(&msg) {
                let outnode_coll:Collection<Document> = db.collection("outnodes");
                let filter = doc! {"peerid": new_sync_node.peerid.to_string()};
                let cursor = outnode_coll.find_one(filter).await;
                if let Ok(opt) = cursor {
                    if let None = opt {
                        if !relay_topic_subscribers.contains(&propagation_source) {
                            if connections.contains(&new_sync_node.peerid) && !clients.contains(&new_sync_node.peerid) {
                                clients.push(new_sync_node.peerid);
                            }
                            if relay_topic_subscribers.len() > 0 && clients.len() == 1 {
                                match swarm
                                    .behaviour_mut()
                                    .gossipsub
                                    .publish(relay_topic, "i have a client".as_bytes()) {
                                        Ok(_) => {
                                  
                                        }
                                        Err(_) => write_log("gossipsub publish problem in gossip_messasges(relay announcement - line 71)!")
                                    }
                            }
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
                let query = doc! {"peer_id": outnode.peer_id.to_string()};
                validators_coll.delete_one(query).await.unwrap();
            }

        }
        Err(_) => write_log("convert gossip message to string problem!"),
    } //convert messages to string
}
