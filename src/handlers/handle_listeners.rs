use std::{
    fs::{self, File, OpenOptions},
    io::{stdout, BufWriter, Write},
};

use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};
use libp2p::{Multiaddr, PeerId};

pub fn handle(address: Multiaddr, local_peer_id: PeerId, my_addresses: &mut Vec<String>) {
    let my_full_addr = format!("{}/p2p/{}", address, local_peer_id);
    execute!(
        stdout(),
        SetForegroundColor(Color::Green),
        Print("Your Full Address:\n".bold()),
        ResetColor
    )
    .unwrap();
    println!("{}", my_full_addr);
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
