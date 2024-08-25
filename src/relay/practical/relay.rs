use libp2p::PeerId;
use rand::seq::IteratorRandom;
use reqwest::Client;
use serde::{Deserialize, Serialize};

//this structure is for knowing that relay is first in the network or not
#[derive(Debug)]
pub struct DialedRelays {
    pub first: First,
    pub relays: Vec<RelayStruct>,
}

#[derive(Debug, PartialEq)]
pub enum First {
    Yes,
    No,
}

impl DialedRelays {
    pub fn new<'a>(first: First, relays: Vec<RelayStruct>) -> Self {
        Self { first, relays }
    }
}
// =====================================================================

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct RelayStruct {
    pub peerid: Option<PeerId>,
    pub wallet: String,
    pub addr: String,
}

impl RelayStruct {
    //make a new relay
    pub fn new(peerid: Option<PeerId>, wallet: String, addr: String) -> Self {
        let relay = Self {
            peerid,
            wallet,
            addr,
        };
        relay
    }

    //return a connected relay ip address as random
    pub fn ip_adress<'a>(dialed_relays: &mut DialedRelays) -> Result<String, &'a str> {
        match dialed_relays.relays.iter().choose(&mut rand::thread_rng()) {
            Some(random_relay) => {
                let trim_address = random_relay.addr.trim_start_matches("/ip4/"); //remove /ip4/ from p2p address
                let split_address = trim_address.split("/").next().unwrap(); // remove all char after first '/'
                Ok(split_address.to_string())
            }
            None => Err("There is no any relays!"),
        }
    }

    pub async fn delete_req<'a>(&self, dialed_relays: &mut DialedRelays) -> Result<(), &'a str> {
        //find relay in relay_number that are relays contacted with they then remove relay from that
        let index = dialed_relays
            .relays
            .iter()
            .position(|relay| relay == self)
            .unwrap();
        dialed_relays.relays.remove(index);
        //post relay address and ip to Centichain server for remove these from server
        let client = Client::new();
        match client
            .delete(format!(
                "https://centichain.org/api/relays?addr={}",
                self.addr
            ))
            .send()
            .await
        {
            Ok(_) => {
                let ip = self.addr.trim_start_matches("/ip4/");
                let ip = ip.split("/").next().unwrap();
                match client
                    .delete(format!("https://centichain.org/api/relays?addr={}", ip))
                    .send()
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(_) => Err("Deleting relay's IP from server problem-(tools/relay 155)"),
                }
            }
            Err(_) => Err("Deleting relay from server problem-(tools/relay 159)"),
        }
    }
}
