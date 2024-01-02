use std::{fs::File, io::{BufReader, BufRead, Write}, env::consts::OS};

use libp2p::PeerId;
use reqwest::Client;

//remove peer from relays.dat file when it disconnected
pub async fn remove_peer(peerid: PeerId) {
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
            client.post("https://centichain.org/api/rmaddr").body(line).send().await.unwrap();
        }
    }
    let mut writer = File::create(relay_path).unwrap();
    for line in lines {
        writeln!(writer, "{}", line).unwrap();
    }
}