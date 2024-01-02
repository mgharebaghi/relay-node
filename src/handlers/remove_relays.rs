use std::{
    env::consts::OS,
    fs::File,
    io::{BufRead, BufReader, Write},
};

use libp2p::PeerId;
use reqwest::Client;

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
        let line = i.unwrap();
        if !line.contains(&peerid.to_string()) {
            lines.push(line);
        } else {
            let client = Client::new();
            client
                .post("https://centichain.org/api/rmaddr")
                .body(line.clone())
                .send()
                .await
                .unwrap();
            let ip = my_addresses[0].trim_start_matches("/ip4/");
            let ip = ip.split("/").next().unwrap();
            println!("{}", ip);
            if !line.contains(ip) {
                let ip = line.trim_start_matches("/ip4/");
                let ip = ip.split("/").next().unwrap();
                client
                    .post("https://centichain.org/api/rmrpc")
                    .body(ip.to_string())
                    .send()
                    .await
                    .unwrap();
            }
        }
    }
    let mut writer = File::create(relay_path).unwrap();
    for line in lines {
        writeln!(writer, "{}", line).unwrap();
    }
}
