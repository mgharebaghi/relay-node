use std::{
    env::consts::OS,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
};

use libp2p::PeerId;

pub fn get_addresses(
    addresses: Vec<String>,
    local_peer_id: PeerId,
    my_addresses: &mut Vec<String>,
) {
    let mut relay_path = "";
    if OS == "linux" {
        relay_path = "/etc/relays.dat";
    } else if OS == "windows" {
        relay_path = "relays.dat";
    }

    let r_relay_file = File::open(relay_path).unwrap();
    let reader = BufReader::new(r_relay_file);
    let mut old_addresses = Vec::new();
    for i in reader.lines() {
        match i {
            Ok(line) => {
                old_addresses.push(line);
            }
            Err(_) => {}
        }
    }
    let relays_file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(relay_path)
        .unwrap();
    let mut buf_writer = BufWriter::new(relays_file);
    for i in addresses {
        if !i.contains(&local_peer_id.to_string()) && !old_addresses.contains(&i) {
            writeln!(buf_writer, "{}", i).unwrap();
        }

        if !my_addresses.contains(&i) {
            my_addresses.push(i);
        }
    }
}
