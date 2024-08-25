use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use events::handler::State;
use mongodb::Database;
use practical::swarm::CentichainBehaviour;
use sp_core::ed25519::Public;
use tools::create_log::write_log;

pub mod events;
pub mod practical;
pub mod tools;

pub struct Relay;

impl Relay {
    pub async fn start(db: &Database) {
        //try to open wallet file to get wallet address of relay
        //it's important for handshaking requests from validators
        let wallet_file = File::open("/etc/wallet.dat");
        let mut wallet_addr = String::new();

        match wallet_file {
            Ok(file) => {
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    let text = line.unwrap();
                    if text.trim().len() > 0 {
                        wallet_addr.push_str(&text);
                    }
                }
                let wallet: Public = wallet_addr.parse().unwrap();
                loop {
                    let (mut swarm, peerid) = CentichainBehaviour::new().await;
                    match CentichainBehaviour::dial(&mut swarm).await {
                        Ok(mut relay_number) => {
                            //handle state of events of network
                            State::handle(&mut swarm, &db, &mut relay_number, &peerid, &wallet)
                                .await;
                        }
                        Err(e) => {
                            write_log(e);
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                write_log(&e.to_string());
                std::process::exit(404)
            }
        }
    }
}
