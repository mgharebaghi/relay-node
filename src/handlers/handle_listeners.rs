use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
};

use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};

use super::create_log::write_log;

#[derive(Debug, Serialize, Deserialize)]
struct Addresses {
    addr: Vec<String>,
}

pub async fn handle(address: Multiaddr, local_peer_id: PeerId, my_addresses: &mut Vec<String>) {
    let my_full_addr = format!("{}/p2p/{}", address, local_peer_id);
    fs::write("/etc/myaddress.dat", my_full_addr.clone()).unwrap();
    send_addr_to_server(my_full_addr.clone()).await;
    my_addresses.push(my_full_addr);
}

async fn send_addr_to_server(full_addr: String) {
    let os = std::env::consts::OS;
    let mut path = "";
    if os == "linux" {
        path = "/etc/relays.dat"
    } else if os == "windows" {
        path = "relays.dat"
    } else {
    };

    let client = reqwest::Client::new();
    let res = client
        .post("https://centichain.org/api/relays")
        .body(full_addr)
        .send()
        .await;

    match res {
        Ok(response) => {
            let deserialize_res: Addresses =
                serde_json::from_str(&response.text().await.unwrap()).unwrap();

            for addr in deserialize_res.addr {
                let exists = fs::metadata(path).is_ok();

                if exists {
                    let mut prev_addresses = Vec::new();
                    let read = File::open(path).unwrap();
                    let reader = BufReader::new(read);
                    for i in reader.lines() {
                        let addr = i.unwrap();
                        prev_addresses.push(addr);
                    } //save addresses to prev_addresses for check new addresses

                    let file = OpenOptions::new()
                        .write(true)
                        .append(true)
                        .open(path)
                        .unwrap();
                    let mut buf_writer = BufWriter::new(&file);
                    if !prev_addresses.contains(&addr) {
                        writeln!(buf_writer, "{}", addr).unwrap();
                    }
                } else {
                    File::create(path).unwrap();
                    let file = OpenOptions::new().write(true).append(true).open(path);
                    match file {
                        Ok(relays_file) => {
                            let mut buf_writer = BufWriter::new(&relays_file);
                            writeln!(buf_writer, "{}", addr).unwrap();
                        }
                        Err(e) => println!("{}", e),
                    }
                }
            }
        }
        Err(_) => {
            write_log("coud not get any response for send your address to centichain.org!".to_string())
        }
    }
}
