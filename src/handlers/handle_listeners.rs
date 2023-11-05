use std::{
    fs::{self, File, OpenOptions},
    io::{stdout, BufRead, BufReader, BufWriter, Write},
};

use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Addresses {
    addr: Vec<String>,
}

pub async fn handle(address: Multiaddr, local_peer_id: PeerId, my_addresses: &mut Vec<String>) {
    let my_full_addr = format!("{}/p2p/{}", address, local_peer_id);
    send_addr_to_server(my_full_addr.clone()).await;
    my_addresses.push(my_full_addr);
}

async fn send_addr_to_server(full_addr: String) {
    let os = std::env::consts::OS;
    let client = reqwest::Client::new();
    let res = client
        .post("https://centichain.org:3002/relays")
        .body(full_addr)
        .send()
        .await
        .unwrap();
    let deserialize_res: Addresses = serde_json::from_str(&res.text().await.unwrap()).unwrap();

    for addr in deserialize_res.addr {
        let exists = match os {
            "linux" => fs::metadata("/etc/relays.dat").is_ok(),
            "windows" => fs::metadata("relays.dat").is_ok(),
        };

        if exists {
            let mut prev_addresses = Vec::new();
            let read = match os {
                "linux" => File::create("/etc/relays.dat").unwrap(),
                "windows" => File::create("relays.dat").unwrap(),
            };
            let reader = BufReader::new(read);
            for i in reader.lines() {
                let addr = i.unwrap();
                prev_addresses.push(addr);
            } //save addresses to prev_addresses for check new addresses

            let file = match os {
                "linux" => OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open("/etc/relays.dat")
                    .unwrap(),
                "windows" => OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open("relays.dat")
                    .unwrap(),
            };
            let mut buf_writer = BufWriter::new(&file);
            if !prev_addresses.contains(&addr) {
                writeln!(buf_writer, "{}", addr).unwrap();
            }
        } else {
            match os {
                "linux" => File::create("/etc/relays.dat").unwrap(),
                "windows" => File::create("relays.dat").unwrap(),
            }
            let file = match os {
                "linux" => OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open("/etc/relays.dat")
                    .unwrap(),
                "windows" => OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open("relays.dat")
                    .unwrap(),
            };
            let mut buf_writer = BufWriter::new(&file);
            writeln!(buf_writer, "{}", addr).unwrap();
        }
    }
}
