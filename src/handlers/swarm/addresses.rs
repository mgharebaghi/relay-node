use std::net::TcpStream;

use libp2p::{futures::StreamExt, Multiaddr, Swarm};
use mongodb::{
    bson::{doc, from_document, to_document, Document},
    Collection, Database,
};

use rand::seq::SliceRandom;

use serde::Deserialize;

use crate::handlers::{practical::relay::{DialedRelays, First, Relay}, tools::create_log::write_log};

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
    pub async fn get<'a>(db: &Database) -> Result<(), &'a str> {
        let response = reqwest::get("https://centichain.org/api/relays").await;
        match response {
            Ok(data) => {
                let collection: Collection<Document> = db.collection("relays");
                match serde_json::from_str::<Addresses>(&data.text().await.unwrap()) {
                    Ok(res) => {
                        if res.status == "success" && res.data.len() > 0 {
                            for relay in res.data {
                                let new_relay = Relay::new(None, String::new(), relay.addr);
                                let doc = to_document(&new_relay).unwrap();
                                collection.insert_one(doc).await.unwrap();
                            }
                            Ok(())
                        } else {
                            Err("first")
                        }
                    }
                    Err(_e) => Err("Error while cast response to json - addresses(46)"),
                }
            }
            Err(_) => Err("Error from getting data - addresses(49)"),
        }
    }

    pub async fn contact<'a>(
        swarm: &mut Swarm<CentichainBehaviour>,
        db: &Database,
    ) -> Result<DialedRelays, &'a str> {
        //check internet connection and if it connection is stable then start dial with relays as random
        write_log("Check your internet...");
        let internet_connection = TcpStream::connect("8.8.8.8:53");

        if let Ok(_) = internet_connection {
            write_log("Your internet is connected");
            //check count of relays and if there are any relays in the network then start dialing to a random relay
            write_log("Checking for relays...");
            let collection: Collection<Document> = db.collection("relays");
            let count_docs = collection.count_documents(doc! {}).await;
            match count_docs {
                Ok(count) => {
                    if count > 0 {
                        Self::contacting(collection, swarm, db).await
                    } else {
                        match Self::get(&db).await {
                            Ok(_) => Self::contacting(collection, swarm, db).await,
                            Err(e) => {
                                if e == "first" {
                                    write_log(
                                        "You Are First Node In The Centichain Network, Welcome:)",
                                    );
                                    let dialed_relays = DialedRelays::new(First::Yes, Vec::new());
                                    Ok(dialed_relays)
                                } else {
                                    Err(e)
                                }
                            }
                        }
                    }
                }
                Err(_) => Err("problem in counting of database documents-(addresses-75)"),
            }
        } else {
            Err("internet connection lost, please check your internet-(addressess-78)")
        }
    }

    async fn contacting<'a>(
        collection: Collection<Document>,
        swarm: &mut Swarm<CentichainBehaviour>,
        db: &Database,
    ) -> Result<DialedRelays, &'a str> {
        write_log("Relays found, Start dialing...");
        let mut relays: Vec<Relay> = Vec::new();
        let mut cursor = collection.find(doc! {}).await.unwrap();

        while let Some(doc) = cursor.next().await {
            let relay: Relay = from_document(doc.unwrap()).unwrap();
            relays.push(relay);
        }

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
        //delete from DB
        let relay_coll: Collection<Document> = db.collection("relay");
        match relay_coll.delete_many(doc! {}).await {
            Ok(_) => {
                for relay in &random_relays {
                    let deleted = collection
                        .delete_one(doc! {"addr": relay.addr.to_string()})
                        .await;
                    //if deleted was ok then dialing will satrts after delete previous connected relay in realy collection

                    if let Ok(_) = deleted {
                        let doc = to_document(&relay).unwrap();

                        match relay_coll.insert_one(doc).await {
                            Ok(_) => {
                                swarm
                                    .dial(relay.addr.parse::<Multiaddr>().unwrap())
                                    .unwrap();
                                write_log(&format!("Dialing with: {}", relay.addr))
                            }
                            Err(_) => {
                                is_err.get_or_insert(
                                    "error from insert of relay into mongodb-(swarm/addresses 133)",
                                );
                                break;
                            }
                        }
                    } else {
                        is_err.get_or_insert(
                            "random relay has problem for deleting-(swarm/dialing 147)",
                        );
                        break;
                    }
                }
            }
            Err(_) => {
                is_err.get_or_insert("Deleting connected relays problem-(swarm/addresses 141)");
            }
        }

        if is_err.is_none() {
            let dialed_relays = DialedRelays::new(First::No, random_relays);
            Ok(dialed_relays)
        } else {
            Err(is_err.unwrap())
        }
    }
}
