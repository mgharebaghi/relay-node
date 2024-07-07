use std::{
    env::consts::OS,
    fs::File,
    io::{BufRead, BufReader, Write},
};

use libp2p::PeerId;
use reqwest::Client;

use super::create_log::write_log;

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
        match i {
            Ok(line) => {
                if !line.contains(&peerid.to_string()) {
                    lines.push(line);
                } else {
                    let client = Client::new();
                    let post_rm_addr = client
                        .delete(format!(
                            "https://centichain.org/api/relays?addr={}",
                            line.clone()
                        ))
                        .send()
                        .await
                        .is_ok();

                    if post_rm_addr {
                        let myaddr_file = File::open("/etc/myaddress.dat").unwrap();
                        let addr_read = BufReader::new(myaddr_file);
                        let mut my_ip = String::new();
                        for i in addr_read.lines() {
                            let addr = i.unwrap();
                            let ip = addr.trim_start_matches("/ip4/");
                            let ip = ip.split("/").next().unwrap();
                            my_ip.push_str(ip);
                        }
                        if !line.contains(&my_ip) {
                            let ip = line.trim_start_matches("/ip4/");
                            let ip = ip.split("/").next().unwrap();
                            match client
                                .delete(format!("https://centichain.org/api/rpc?addr={}", ip))
                                .send()
                                .await
                            {
                                Ok(_) => {}
                                Err(_) => {
                                    write_log("can not post the ip for remove rpc! remove_relays(line 58)");
                                }
                            }
                        }
                    } else {
                        write_log("post remove address problem! remove_relays(line 63)");
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
