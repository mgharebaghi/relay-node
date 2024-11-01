use std::time::Duration;

use futures::StreamExt;
use libp2p::{
    request_response::{Event, Message},
    swarm::SwarmEvent,
    Multiaddr, PeerId, Swarm,
};
use mongodb::{
    bson::{doc, from_document, Document},
    Collection, Database,
};
use tokio::time::sleep;

use crate::relay::{
    events::{addresses::Listeners, requests::Requests},
    practical::{swarm::Req, transaction::Transaction},
    tools::create_log::write_log,
};

use super::middlegossiper_swarm::{MiddleSwarmConf, MyBehaviour, MyBehaviourEvent};

pub struct MiddleGossipper;

impl MiddleGossipper {
    pub async fn checker(db: &Database) {
        //dialing to relay that is in the radsress collection(raddress means Relay Address)
        let mut swarm = MyBehaviour::new().await;
        sleep(Duration::from_secs(60)).await; //delay to save addresses of relay to DB
        let collection: Collection<Document> = db.collection("raddress");
        let addr_doc = collection.find_one(doc! {}).await.unwrap().unwrap(); // find relay address from database
        let addresses: Listeners = from_document(addr_doc).unwrap(); // deserialize document to listener structure
        let dial_address: Multiaddr = addresses.p2p.parse().unwrap(); // pars p2p of listener to multi address for dialing
        swarm.dial(dial_address).unwrap(); // dialing

        let mut connected_id = String::new();
        loop {
            match swarm.select_next_some().await {
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    write_log("Middlegossipper connection stablished.");

                    connected_id.push_str(&peer_id.to_string());

                    match Self::wathcing(db, &mut swarm, peer_id).await {
                        Ok(_) => {}
                        Err(e) => {
                            write_log(e);
                            std::process::exit(0);
                        }
                    }
                }

                SwarmEvent::OutgoingConnectionError { .. } => {
                    write_log("Middlegossiper dialing error!");
                    std::process::exit(0)
                }

                SwarmEvent::ConnectionClosed { .. } => {
                    write_log("Middlegossiper connection closed!");
                }

                SwarmEvent::Behaviour(mybehaviour) => match mybehaviour {
                    MyBehaviourEvent::Gossipsub(event) => match event {
                        _ => {}
                    },
                    MyBehaviourEvent::ReqRes(event) => match event {
                        Event::Message { message, .. } => match message {
                            Message::Response { .. } => {
                                let peer_id: PeerId = connected_id.parse().unwrap();

                                match Self::wathcing(db, &mut swarm, peer_id).await {
                                    Ok(_) => {}
                                    Err(e) => {
                                        write_log(e);
                                        std::process::exit(0);
                                    }
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                },
                _ => {}
            }
        }
    }

    //wathcing mongo
    async fn wathcing<'a>(
        db: &Database,
        swarm: &mut Swarm<MyBehaviour>,
        peer_id: PeerId,
    ) -> Result<(), &'a str> {
        let transactions_coll: Collection<Document> = db.collection("transactions");
        let pipeline = vec![doc! { "$match": {
            "operationType": "insert"
        }}];
        let mut watchin = transactions_coll
            .watch()
            .pipeline(pipeline)
            .await
            .expect("error in mongodb watching");

        if let Some(change) = watchin.next().await {
            match change {
                Ok(data) => {
                    if let Some(doc) = data.full_document {
                        let transaction: Transaction = from_document(doc).unwrap();
                        let str_trx =
                            serde_json::to_string(&Requests::Transaction(transaction)).unwrap();
                        let request = Req { req: str_trx };
                        swarm
                            .behaviour_mut()
                            .req_res
                            .send_request(&peer_id, request);
                        Ok(())
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err("Error in watching of mongodb-line(138)"),
            }
        } else {
            Ok(())
        }
    }
}
