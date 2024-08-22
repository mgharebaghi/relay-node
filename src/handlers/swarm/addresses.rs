use std::net::TcpStream;

use libp2p::{Multiaddr, Swarm};

use rand::seq::SliceRandom;

use serde::Deserialize;

use crate::handlers::{
    practical::relay::{DialedRelays, First, Relay},
    tools::create_log::write_log,
};

use super::CentichainBehaviour;

#[derive(Debug, Deserialize)]
pub struct Addresses {
    status: String,
    data: Vec<Address>,
}

#[derive(Debug, Deserialize)]
struct Address {
    addr: String,
}

impl Addresses {
    pub async fn get<'a>() -> Result<Vec<Relay>, &'a str> {
        let response = reqwest::get("https://centichain.org/api/relays").await;
        match response {
            Ok(data) => match serde_json::from_str::<Addresses>(&data.text().await.unwrap()) {
                Ok(res) => {
                    let mut relays = Vec::new();
                    if res.status == "success" && res.data.len() > 0 {
                        for relay in res.data {
                            relays.push(Relay::new(None, String::new(), relay.addr));
                        }
                        Ok(relays)
                    } else {
                        Err("first")
                    }
                }
                Err(_e) => Err("Error while cast response to json - addresses(46)"),
            },
            Err(_) => Err("Error from getting data - addresses(49)"),
        }
    }

    pub async fn contact<'a>(
        swarm: &mut Swarm<CentichainBehaviour>,
    ) -> Result<DialedRelays, &'a str> {
        //check internet connection and if it connection is stable then start dial with relays as random
        write_log("Check your internet...");
        let internet_connection = TcpStream::connect("8.8.8.8:53");

        if internet_connection.is_ok() {
            write_log("Your internet is connected");
            //check count of relays and if there are any relays in the network then start dialing to a random relay
            write_log("Checking for relays...");
            match Self::get().await {
                Ok(relays) => Self::contacting(relays, swarm).await,
                Err(e) => {
                    if e == "first" {
                        write_log("You Are First Node In The Centichain Network, Welcome:)");
                        let dialed_relays = DialedRelays::new(First::Yes, Vec::new());
                        Ok(dialed_relays)
                    } else {
                        Err(e)
                    }
                }
            }
        } else {
            Err("internet connection lost, please check your internet-(addressess-78)")
        }
    }

    async fn contacting<'a>(
        relays: Vec<Relay>,
        swarm: &mut Swarm<CentichainBehaviour>,
    ) -> Result<DialedRelays, &'a str> {
        write_log("Relays found, Start dialing...");
        //choos 6 relays as random for dialing
        let mut random_relays: Vec<Relay> = Vec::new();
        if relays.len() > 6 {
            while random_relays.len() <= 6 {
                let random_relay = relays.choose(&mut rand::thread_rng()).unwrap();
                if !random_relays.contains(&random_relay) {
                    random_relays.push(random_relay.clone());
                }
            }
        } else {
            for i in 0..relays.len() {
                random_relays.push(relays[i].clone());
            }
        }

        let mut is_err: Option<&str> = None;
        for relay in &random_relays {
            match swarm.dial(relay.addr.parse::<Multiaddr>().unwrap()) {
                Ok(_) => {}
                Err(_) => {
                    is_err.get_or_insert("Dialing Error!");
                }
            }
            write_log(&format!("Dialing with: {}", relay.addr))
        }

        if is_err.is_none() {
            let dialed_relays = DialedRelays::new(First::No, random_relays);
            Ok(dialed_relays)
        } else {
            Err(is_err.unwrap())
        }
    }
}
