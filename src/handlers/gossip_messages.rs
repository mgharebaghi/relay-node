use std::{fs::{File, OpenOptions}, io::{BufReader, BufRead, BufWriter, Write}, env::consts::OS};

use libp2p::{PeerId, gossipsub::{Message, IdentTopic}, Swarm};

use crate::handlers::structures::ImSync;

use super::structures::CustomBehav;

pub fn handle_gossip_message(
    propagation_source: PeerId,
    message: Message,
    local_peer_id: PeerId,
    clients: &mut Vec<PeerId>,
    relays: &mut Vec<PeerId>,
    swarm: &mut Swarm<CustomBehav>,
    relay_topic: IdentTopic,
    connections: &mut Vec<PeerId>,
    relay_topic_subscribers: &mut Vec<PeerId>,
    my_addresses: &mut Vec<String>,
) {
    let msg = String::from_utf8(message.data.clone()).unwrap(); //convert messages to string

    if let Ok(addresses) = serde_json::from_str::<Vec<String>>(&msg) {
        let mut relay_path = "";
        if OS == "linux" {
            relay_path = "/etc/relays.dat";
        } else if OS == "windows" {
            relay_path = "relays.dat";
        }

        let r_relay_file = File::open(relay_path).unwrap();
        let reader = BufReader::new(r_relay_file);
        let mut old_addresses = Vec::new();
        for i in reader.lines() {
            let line = i.unwrap();
            old_addresses.push(line);
        }
        let relays_file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(relay_path)
            .unwrap();
        let mut buf_writer = BufWriter::new(relays_file);
        for i in addresses {
            if !i.contains(&local_peer_id.to_string()) && !old_addresses.contains(&i) {
                writeln!(buf_writer, "{}", i).unwrap();
            }

            if !my_addresses.contains(&i) {
                my_addresses.push(i);
            }
        }
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
                swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(relay_topic, "i have a client".as_bytes())
                    .unwrap();
            }
        }
    }

    if msg == "i have a client".to_string() && connections.contains(&propagation_source) {
        if !relays.contains(&propagation_source) {
            relays.push(propagation_source);
        }
    }

    if msg == "i dont have any clients".to_string() && relays.contains(&propagation_source) {
        let index = relays
            .iter()
            .position(|relay| *relay == propagation_source)
            .unwrap();
        relays.remove(index);
    }
}