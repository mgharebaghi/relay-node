use std::{
    env::consts::OS,
    fs::File,
    io::{BufRead, BufReader, Write},
};

use libp2p::PeerId;
use reqwest::Client;

use super::create_log::write_log;

//remove peer from relays.dat file when it disconnected
pub async fn remove_peer(peerid: PeerId, my_addresses: &mut Vec<String>) {
    let mut relay_path = "";
    if OS == "linux" {
        relay_path = "/etc/relays.dat";
    } else if OS == "windows" {
        relay_path = "relays.dat";
    }
    let file = File::open(relay_path).unwrap();
    let reader = BufReader::new(&file);
    let mut lines = Vec::new();
    for i in reader.lines() {
        match i {
            Ok(line) => {
                if !line.contains(&peerid.to_string()) {
                    lines.push(line);
                } else {
                    let client = Client::new();
                    let post_rm_addr = client
                        .post("https://centichain.org/api/rmaddr")
                        .body(line.clone())
                        .send()
                        .await
                        .is_ok();

                    if post_rm_addr {
                        if !line.contains(&my_addresses[0]) {
                            let ip = line.trim_start_matches("/ip4/");
                            let ip = ip.split("/").next().unwrap();
                            match client
                                .post("https://centichain.org/api/rmrpc")
                                .body(ip.to_string())
                                .send()
                                .await
                            {
                                Ok(_) => {}
                                Err(_) => {
                                    write_log("can not post the ip for remove rpc in remove relay section!".to_string());
                                }
                            }
                        }
                    } else {
                        write_log("post remove address problem!".to_string());
                    }
                }
            }
            Err(_) => {}
        }
    }
    let mut writer = File::create(relay_path).unwrap();
    for line in lines {
        writeln!(writer, "{}", line).unwrap();
    }
}
