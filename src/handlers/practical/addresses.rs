use libp2p::{Multiaddr, PeerId};
use mongodb::{
    bson::{doc, to_document, Document},
    Collection, Database,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::handlers::tools::create_log::write_log;

#[derive(Debug, Serialize, Deserialize)]
pub struct Listeners {
    pub p2p: String,
    ip: String,
}

#[derive(Debug, Serialize)]
struct PostListener {
    addr: String,
}

impl PostListener {
    //generate new listeners
    fn new(addr: &String) -> Self {
        Self { addr: addr.clone() }
    }
}

impl Listeners {
    //generate new listener structure with get new listener address and peer id
    pub async fn new<'a>(
        listener: &Multiaddr,
        peerid: &PeerId,
        db: &'a Database,
    ) -> Result<Self, &'a str> {
        let p2p = format!("{}/p2p/{}", listener.to_string(), peerid);
        let public_ip = public_ip::addr().await;
        match public_ip {
            Some(ip) => {
                if p2p.contains(&ip.to_string()) {
                    let new_listener = Self {
                        p2p,
                        ip: ip.to_string(),
                    };
                    let collection: Collection<Document> = db.collection("raddress");
                    let listener_to_doc = to_document(&new_listener).unwrap();
                    collection.delete_many(doc! {}).await.unwrap();
                    collection.insert_one(listener_to_doc).await.unwrap();
                    Ok(new_listener)
                } else {
                    Err("")
                }
            }
            None => Err("You don't have any public ips!"),
        }
    }

    //post p2p address and ip address to the server as relay address and rpc address
    pub async fn post<'a>(&self) -> Result<(), ()> {
        let client = Client::new();

        match client
            .post("https://centichain.org/api/relays")
            .json(&PostListener::new(&self.p2p))
            .send()
            .await
        {
            Ok(_) => {
                match client
                    .post("https://centichain.org/api/rpc")
                    .json(&PostListener::new(&self.ip))
                    .send()
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(e) => Err(write_log(&format!(
                        "Sending ip address to server error: {}",
                        e
                    ))),
                }
            }
            Err(e) => Err(write_log(&format!(
                "Sending p2p address to server error: {}",
                e
            ))),
        }
    }
}
