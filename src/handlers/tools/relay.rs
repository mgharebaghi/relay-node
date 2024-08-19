use libp2p::PeerId;
use mongodb::{
    bson::{doc, from_document, Document},
    Collection, Database,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

//this structure is for knowing that relay is first in the network or not
#[derive(Debug)]
pub struct RelayNumber {
    pub first: First,
    pub relays: Vec<Relay>,
}

#[derive(Debug)]
pub enum First {
    Yes,
    No,
}

impl RelayNumber {
    pub fn new<'a>(first: First, relays: Vec<Relay>) -> Self {
        Self { first, relays }
    }
}
// =====================================================================

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Relay {
    pub peerid: Option<PeerId>,
    pub wallet: String,
    pub addr: String,
}

impl Relay {
    //make a new relay
    pub fn new(peerid: Option<PeerId>, wallet: String, addr: String) -> Self {
        let relay = Self {
            peerid,
            wallet,
            addr,
        };
        relay
    }

    //update relay
    pub async fn update(
        &mut self,
        db: &Database,
        peerid: Option<PeerId>,
        wallet: Option<String>,
    ) -> Result<Self, &str> {
        let collection: Collection<Document> = db.collection("relay");
        let mut update = None;
        if peerid.is_some() {
            update.get_or_insert(doc! {"$set": {"peerid": peerid.unwrap().to_string()}});
            self.peerid.get_or_insert(peerid.unwrap());
        } else {
            self.wallet.push_str(&wallet.clone().unwrap());
            update.get_or_insert(doc! {"$set": {"wallet": wallet.unwrap()}});
        };
        match collection.update_one(doc! {}, update.unwrap()).await {
            Ok(_) => {
                let find_doc = collection.find_one(doc! {}).await;
                if let Ok(doc) = find_doc {
                    let relay: Relay = from_document(doc.unwrap()).unwrap();
                    Ok(relay)
                } else {
                    Err("Error while finding connected relay-(events/relay 45)")
                }
            }
            Err(_) => Err("Updating relay error-(events/relay 48)"),
        }
    }

    //return connected relay ip address
    pub async fn ip_adress<'a>(db: &'a Database) -> Result<String, &'a str> {
        //check relay document in relay collection of Centichain Database and if it has a document then continue else return an error
        let collection: Collection<Document> = db.collection("relay");
        let relay_doc = collection.find_one(doc! {}).await;
        match relay_doc {
            Ok(document) => {
                if let Some(doc) = document {
                    let relay: Self = from_document(doc).unwrap(); //deserialize document to a Relay struct for get its address
                    let p2p_addr = relay.addr; //get relay's p2p address from its document
                    let trim_addr = p2p_addr.trim_start_matches("/ip4/");
                    let split_addr = trim_addr.split("/").next().unwrap(); //split the p2p address for find relay's ip address
                    Ok(split_addr.to_string())
                } else {
                    Err("There is no any Relay in Database!")
                }
            }
            Err(_e) => Err("Find Relay Document Problem-(generator/relay 69)"),
        }
    }

    // pub async fn find<'a>(db: &'a Database) -> Result<Self, &'a str> {
    //     let collection: Collection<Document> = db.collection("relay");
    //     let query = collection.find_one(doc! {}).await;
    //     match query {
    //         Ok(opt) => {
    //             if let Some(doc) = opt {
    //                 let relay: Self = from_document(doc).unwrap();
    //                 Ok(relay)
    //             } else {
    //                 Err("There is no any relays in relay collection of database!")
    //             }
    //         }
    //         Err(_) => Err("Qurying relay problem! please check your mongodb."),
    //     }
    // }

    pub async fn remove<'a>(
        &self,
        db: &'a Database,
        relay_number: &mut RelayNumber,
    ) -> Result<(), &'a str> {
        //delete relay from database at first
        let collection: Collection<Document> = db.collection("relay");
        match collection.delete_one(doc! {"addr": &self.addr}).await {
            Ok(_) => {
                //find relay in relay_number that are relays contacted with they then remove relay from that
                let index = relay_number
                    .relays
                    .iter()
                    .position(|relay| relay == self)
                    .unwrap();
                relay_number.relays.remove(index);
                //post relay address and ip to Centichain server for remove these from server
                let client = Client::new();
                match client.delete(format!("https://centichain.org/api/relays?addr={}", self.addr)).send().await {
                    Ok(_) => {
                        let ip = self.addr.trim_start_matches("/ip4/");
                        let ip = ip.split("/").next().unwrap();
                        match client.delete(format!("https://centichain.org/api/relays?addr={}", ip)).send().await {
                            Ok(_) => Ok(()),
                            Err(_) => Err("Deleting relay's IP from server problem-(tools/relay 135)")
                        }
                    }
                    Err(_) => Err("Deleting relay from server problem-(tools/relay 138)")
                }
            }
            Err(_) => Err("Deleting relay problem-(tools/relay 141)"),
        }
    }
}
