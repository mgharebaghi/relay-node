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
mod responses;
mod send_address;
mod send_response;
pub mod structures;
use handle_events::events;
use structures::CustomBehav;
pub mod create_log;
mod db_connection;
mod handle_messages;
mod recieved_block;
mod check_trx;
mod reciept;
mod nodes_sync_announce;


use crate::handlers::create_log::write_log;

use self::structures::{Channels, FullNodes};
//handle streams that come to swarm events and relays.dat file to add or remove addresses
pub async fn handle_streams(
    local_peer_id: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    clients_topic: IdentTopic,
    my_addresses: &mut Vec<String>,
    channels: &mut Vec<Channels>,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
    connections: &mut Vec<PeerId>,
    relay_topic_subscribers: &mut Vec<PeerId>,
    client_topic_subscriber: &mut Vec<PeerId>,
    wallet: &mut String,
    leader: &mut String, 
    fullnodes: &mut Vec<FullNodes>
) {
    loop {
        let relays_file_exist = fs::metadata("/etc/relays.dat").is_ok();
        if relays_file_exist {
            let file = File::open("/etc/relays.dat").unwrap();
            let reader = BufReader::new(&file);
            let mut dial_addresses = Vec::new();
            for i in reader.lines() {
                let addr = i.unwrap();
                if addr.trim().len() > 0 {
                    let addresses: Multiaddr = addr.parse().unwrap();
                    if !addresses.to_string().contains(&local_peer_id.to_string()) {
                        dial_addresses.push(addresses);
                    }
                }
            }
            if dial_addresses.len() > 0 {
                let rnd_dial_addr = dial_addresses
                    .choose(&mut rand::thread_rng())
                    .unwrap()
                    .clone();
                match swarm.dial(rnd_dial_addr.clone()) {
                    Ok(_) => {
                        write_log(format!("Dialing with: {}", rnd_dial_addr));
                    }
                    Err(_) => {
                        write_log("dialing problem!".to_string());
                    }
                }
            }
        }

        events(
            swarm,
            local_peer_id,
            my_addresses,
            clients,
            channels,
            relays,
            clients_topic.clone(),
            relay_topic.clone(),
            &mut connections.clone(),
            relay_topic_subscribers,
            client_topic_subscriber,
            wallet,
            leader,
            fullnodes
        )
        .await;
    }
}
