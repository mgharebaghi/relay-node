use std::{fs::{OpenOptions, self, File}, io::{BufWriter, Write}};

use libp2p::{Multiaddr, PeerId};
use log::info;

pub fn handle(
    address: Multiaddr,
    local_peer_id: PeerId,
    my_addresses: &mut Vec<String>,
) {
    let my_full_addr = format!("{}/p2p/{}", address, local_peer_id);
    info!("Your full address:\n{}", my_full_addr);
    let exists = fs::metadata("relays.dat").is_ok();
    if exists {
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open("relays.dat")
            .unwrap();
        let mut buf_writer = BufWriter::new(&file);
        writeln!(buf_writer, "{}", my_full_addr.clone()).unwrap();
    } else {
        File::create("relays.dat").unwrap();
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open("relays.dat")
            .unwrap();
        let mut buf_writer = BufWriter::new(&file);
        let my_full_addr = format!("{}/p2p/{}", address, local_peer_id);
        writeln!(buf_writer, "{}", my_full_addr).unwrap();
    }
    my_addresses.push(my_full_addr);
}
