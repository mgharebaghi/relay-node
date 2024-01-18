use std::{
    env::consts::OS,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
};

use libp2p::{
    gossipsub::{IdentTopic, Message},
    PeerId, Swarm,
};
use reqwest::Client;

use crate::handlers::structures::ImSync;

use super::{create_log::write_log, structures::CustomBehav};

pub async fn handle_gossip_message(
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
    match String::from_utf8(message.data.clone()) {
        Ok(msg) => {
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
                    match i {
                        Ok(line) => {
                            old_addresses.push(line);
                        }
                        Err(_) => {}
                    }
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

            if msg == "i have a client".to_string() && connections.contains(&propagation_source) {
                if !relays.contains(&propagation_source) {
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
        }
        Err(_) => write_log("convert gossip message to string problem!".to_string()),
    } //convert messages to string
}
