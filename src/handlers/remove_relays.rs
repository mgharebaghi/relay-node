use std::{fs::File, io::{BufReader, BufRead, Write}};

use libp2p::PeerId;

//remove peer from relays.dat file when it disconnected
pub fn remove_peer(peerid: PeerId) {
    let file = File::open("relays.dat").unwrap();
    let reader = BufReader::new(&file);
    let mut lines = Vec::new();
    for i in reader.lines() {
        let line = i.unwrap();
        if !line.contains(&peerid.to_string()) {
            lines.push(line);
        }
    }
    let mut writer = File::create("relays.dat").unwrap();
    for line in lines {
        writeln!(writer, "{}", line).unwrap();
    }
}