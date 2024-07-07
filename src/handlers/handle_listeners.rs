use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
};

use libp2p::{Multiaddr, PeerId};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::create_log::write_log;

#[derive(Debug, Serialize, Deserialize)]
struct Addresses {
    addr: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MyAdrress {
    addr: String,
}

#[derive(Debug, Deserialize)]
struct ServRes {
    _status: String,
    _detail: String,
    anothers: Vec<String>,
}

pub async fn handle(address: Multiaddr, local_peer_id: PeerId, my_addresses: &mut Vec<String>) {
    let my_full_addr = format!("{}/p2p/{}", address, local_peer_id);
    fs::write("/etc/myaddress.dat", my_full_addr.clone()).unwrap();
    my_addresses.push(my_full_addr.clone());
}

pub async fn send_addr_to_server(full_addr: String) {
    let os = std::env::consts::OS;
    let mut path = "";
    if os == "linux" {
        path = "/etc/relays.dat"
    } else if os == "windows" {
        path = "relays.dat"
    }

    let myfulladdr = MyAdrress {
        addr: full_addr.clone(),
    };

    let client = reqwest::Client::new();
    let res = client
        .post("https://centichain.org/api/relays")
        .json(&myfulladdr)
        .send()
        .await;

    write_log(&format!("sent addr: {:?}", myfulladdr));

    match res {
        Ok(response) => {
            write_log("my addresses posted to server");
            let mut addresses = String::new();
            match response.json::<ServRes>().await {
                Ok(data) => {
                    if data.anothers.len() > 0 {
                        for addr in data.anothers {
                            addresses.push_str(&addr);
                        }
                    }
                }
                Err(_) => {
                    write_log("centichain server response error! handle_listener(line 48)");
                }
            }

            match serde_json::from_str::<Addresses>(&addresses) {
                Ok(deserialize_res) => {
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
                                Err(e) => write_log(&format!("{}", e)),
                            }
                        }
                    }
                }
                Err(_) => {
                    write_log(
                        "deserializing addresses from server problem! handle_listener(line 90)",
                    );
                }
            }
        }
        Err(_) => write_log("coud not get any response for send your address to centichain.org! handle_listener(line 95)"),
    }

    //send ip address as RPC server
    let trim_my_addr = full_addr.trim_start_matches("/ip4/");
    let my_ip = trim_my_addr.split("/").next().unwrap();
    let rpcaddr = MyAdrress {
        addr: my_ip.to_string(),
    };
    let client = Client::new();
    let res = client
        .post("https://centichain.org/api/rpc")
        .json(&rpcaddr)
        .send()
        .await;

    write_log(&format!("sent RPC: {:?}", rpcaddr));
    match res {
        Ok(_) => {}
        Err(_) => write_log(
            "Can not send your public ip to the server as RPC server! handle_listener(line 110)",
        ),
    }
}
