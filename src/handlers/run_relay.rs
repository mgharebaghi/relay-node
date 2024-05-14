use std::{
    env::consts::OS,
    fs::File,
    io::{BufRead, BufReader},
};

use libp2p::{gossipsub::IdentTopic, PeerId, Swarm};
use mongodb::Database;

use super::{create_log::write_log, listening_dialing::start, swarm_config::CustomBehav};

pub async fn run(swarm: &mut Swarm<CustomBehav>, local_peer_id: PeerId, db: Database) {
    let mut wallet = String::new();
    let mut wallet_path = "";
    if OS == "linux" {
        wallet_path = "/etc/wallet.dat"
    } else if OS == "windows" {
        wallet_path = "wallet.dat"
    };
    let wallet_file = File::open(wallet_path);
    match wallet_file {
        Ok(file) => {
            let reader = BufReader::new(file);
            for addr in reader.lines() {
                let wallet_addr = addr.unwrap();
                if wallet_addr.trim().len() > 0 {
                    wallet.push_str(&wallet_addr);
                }
            }
        }
        Err(_) => {
            write_log("Could not find the wallet address file!");
            std::process::exit(404);
        }
    }

    let mut connections: Vec<PeerId> = Vec::new();
    let mut relay_topic_subscribers: Vec<PeerId> = Vec::new();
    let mut client_topic_subscribers: Vec<PeerId> = Vec::new();
    let mut clients: Vec<PeerId> = Vec::new();
    let mut relays: Vec<PeerId> = Vec::new();
    let mut leader = String::new();
    let mut my_addresses = Vec::new();
    let mut sync = false;
    let mut syncing_blocks = Vec::new();
    let relay_topic = IdentTopic::new("relay");
    let clients_topic = IdentTopic::new("client");

    start(
        local_peer_id,
        swarm,
        clients_topic,
        &mut my_addresses,
        &mut relays,
        &mut clients,
        relay_topic,
        &mut connections,
        &mut relay_topic_subscribers,
        &mut client_topic_subscribers,
        &mut wallet,
        &mut leader,
        &mut sync,
        &mut syncing_blocks,
        db,
    )
    .await;
}
