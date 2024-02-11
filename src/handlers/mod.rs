use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
};

use libp2p::{gossipsub::IdentTopic, Multiaddr, PeerId, Swarm};
use rand::seq::SliceRandom;

mod gossip_messages;
mod handle_events;
mod handle_listeners;
mod outnodes;
mod remove_relays;
mod requests;
mod send_address;
pub mod structures;
use handle_events::events;
use structures::CustomBehav;
mod check_trx;
pub mod create_log;
mod db_connection;
mod get_addresses;
mod handle_messages;
mod nodes_sync_announce;
mod reciept;
mod recieved_block;
mod syncing;

use crate::handlers::create_log::write_log;

use self::structures::{FullNodes, GetGossipMsg};
//handle streams that come to swarm events and relays.dat file to add or remove addresses
pub async fn handle_streams(
    local_peer_id: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    clients_topic: IdentTopic,
    my_addresses: &mut Vec<String>,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
    connections: &mut Vec<PeerId>,
    relay_topic_subscribers: &mut Vec<PeerId>,
    client_topic_subscriber: &mut Vec<PeerId>,
    wallet: &mut String,
    leader: &mut String,
    fullnodes: &mut Vec<FullNodes>,
    sync: &mut bool,
    syncing_blocks: &mut Vec<GetGossipMsg>,
) {
    loop {
        let relays_file_exist = fs::metadata("/etc/relays.dat").is_ok();
        let mut dialed_addr:Vec<String> = Vec::new();
        if relays_file_exist {
            let file = File::open("/etc/relays.dat").unwrap();
            let reader = BufReader::new(&file);
            let mut dial_addresses = Vec::new();
            for i in reader.lines() {
                let addr = i.unwrap();
                if addr.trim().len() > 0 {
                    match addr.parse::<Multiaddr>() {
                        Ok(addresses) => {
                            if !addresses.to_string().contains(&local_peer_id.to_string()) {
                                dial_addresses.push(addresses);
                            }
                        }
                        Err(_) => {}
                    }
                }
            }

            if dial_addresses.len() > 0 {
                if dial_addresses.len() < 9 {
                    for addr in dial_addresses {
                        match swarm.dial(addr.clone()) {
                            Ok(_) => {
                                println!("dialing with: {}", addr);
                                dialed_addr.push(addr.to_string());
                            }
                            Err(_) => {
                                write_log(format!("dialing problem with: {}", addr));
                            }
                        }
                    }
                } else {
                    let mut rnd_relays = Vec::new();
                    while rnd_relays.len() < 9 {
                        let new_rnd = dial_addresses.choose(&mut rand::thread_rng()).unwrap();
                        if !rnd_relays.contains(new_rnd) {
                            rnd_relays.push(new_rnd.clone())
                        }
                    }
                    for addr in rnd_relays {
                        match swarm.dial(addr.clone()) {
                            Ok(_) => {
                                dialed_addr.push(addr.to_string());
                            }
                            Err(_) => {
                                write_log(format!("dialing problem with: {}", addr));
                            }
                        }
                    }
                }
            } else {
                *sync = true
            }
        } else {
            *sync = true
        }

        let listener: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
        swarm.listen_on(listener).unwrap();

        events(
            swarm,
            local_peer_id,
            my_addresses,
            clients,
            relays,
            clients_topic.clone(),
            relay_topic.clone(),
            &mut connections.clone(),
            relay_topic_subscribers,
            client_topic_subscriber,
            wallet,
            leader,
            fullnodes,
            sync,
            &mut dialed_addr,
            syncing_blocks,
        )
        .await;
    }
}
