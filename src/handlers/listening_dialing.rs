use std::{env::consts::OS, fs::{self, File, OpenOptions}, io::{BufRead, BufReader, BufWriter, Write}, net::TcpStream, sync::{Arc, Mutex}};

use libp2p::{gossipsub::IdentTopic, Multiaddr, PeerId, Swarm};
use mongodb::Database;
use rand::seq::SliceRandom;

use super::{create_log::write_log, handle_events::events, handle_listeners::send_addr_to_server, structures::{FullNodes, GetGossipMsg}, swarm_config::CustomBehav, Addresses};

pub async fn start(
    local_peer_id: PeerId,
    swarm: Arc<Mutex<Swarm<CustomBehav>>>,
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
    db: Database
) {
    loop {
        let server_address = "www.centichain.org:80";
        let site_connection = TcpStream::connect(server_address);

        let mut relays_path = "";
        if OS == "windows" {
            relays_path = "relays.dat"
        } else if OS == "linux" {
            relays_path = "/etc/relays.dat"
        }

        match site_connection {
            Ok(_) => get_addresses(relays_path).await,
            Err(_) => write_log(
                "Relay could not connect to centichain.org for get latest relays addresses! mod.rs(line 73)",
            ),
        }
        let mut dialed_addr = dialing(
            relays_path,
            local_peer_id,
            Arc::clone(&swarm),
            sync,
            my_addresses,
        )
        .await;
        let mut im_first = false;
        if dialed_addr.len() == 0 {
            im_first = true;
        }
        write_log(&format!("im first: {}", im_first));
        events(
            Arc::clone(&swarm),
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
            im_first,
            db.clone()
        )
        .await;
    }
}

async fn get_addresses(relays_path: &str) {
    match reqwest::get("https://centichain.org/api/relays")
        .await
    {
        Ok(response) => {
            let addresses: Addresses = response.json().await.unwrap();
            write_log(&format!("addresses:\n{:?}", addresses));
            let path_exist = fs::metadata(relays_path).is_ok();
            if path_exist {
                fs::write(relays_path, "").unwrap();
                let write_file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(relays_path)
                    .unwrap();
                let mut writer = BufWriter::new(write_file);
                for addr in addresses.addr {
                    writeln!(writer, "{}", addr).unwrap();
                }
            } else {
                let relays_file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(relays_path)
                    .unwrap();
                let mut writer = BufWriter::new(relays_file);

                for addr in addresses.addr {
                    writeln!(writer, "{}", addr).unwrap();
                }
            }
        }
        Err(_) => {
            write_log("get relays from server problem! mod.rs(line 138)");
        }
    }
}

pub async fn dialing(
    relays_path: &str,
    local_peer_id: PeerId,
    swarm: Arc<Mutex<Swarm<CustomBehav>>>,
    sync: &mut bool,
    my_addresses: &mut Vec<String>,
) -> Vec<String> {
    let mut swarm = swarm.lock().unwrap();
    let relays_file_exist = fs::metadata(relays_path).is_ok();
    let mut dialed_addr: Vec<String> = Vec::new();
    if relays_file_exist {
        let file = File::open(relays_path).unwrap();
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
            if dial_addresses.len() < 6 {
                for addr in dial_addresses {
                    match swarm.dial(addr.clone()) {
                        Ok(_) => {
                            write_log(&format!("dialing with: {}", addr));
                            dialed_addr.push(addr.to_string());
                        }
                        Err(_) => {
                            write_log(&format!("dialing problem with: {}", addr));
                        }
                    }
                }
            } else {
                let mut rnd_relays = Vec::new();
                while rnd_relays.len() < 6 {
                    let new_rnd = dial_addresses.choose(&mut rand::thread_rng()).unwrap();
                    if !rnd_relays.contains(new_rnd) {
                        rnd_relays.push(new_rnd.clone())
                    }
                }
                for addr in rnd_relays {
                    match swarm.dial(addr.clone()) {
                        Ok(_) => {
                            write_log(&format!("dialing with: {}", addr));
                            dialed_addr.push(addr.to_string());
                        }
                        Err(_) => {
                            write_log(&format!("dialing problem with: {}", addr));
                        }
                    }
                }
            }
        } else {
            if my_addresses.len() > 0 {
                send_addr_to_server(my_addresses[0].clone()).await;
            }
            *sync = true
        }
    } else {
        if my_addresses.len() > 0 {
            send_addr_to_server(my_addresses[0].clone()).await;
        }
        *sync = true
    }

    dialed_addr
}
