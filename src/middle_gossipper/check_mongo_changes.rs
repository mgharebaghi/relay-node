use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use futures::StreamExt;
use libp2p::{gossipsub::IdentTopic, request_response::Event, swarm::SwarmEvent, Multiaddr, Swarm};
use mongodb::{
    bson::{doc, from_document, Document},
    Collection,
};

use crate::handlers::{
    check_trx::handle_transactions, create_log::write_log, db_connection::blockchain_db,
    structures::Transaction,
};

use super::middlegossiper_swarm::{MyBehaviour, MyBehaviourEvent};

pub async fn checker(swarm: &mut Swarm<MyBehaviour>) {
    let mut connected_id = String::new();
    let clients_topic = IdentTopic::new("client");
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                connected_id.push_str(&peer_id.to_string());
            }
            SwarmEvent::OutgoingConnectionError { .. } => {
                let my_addr_file = File::open("/etc/myaddress.dat");
                if let Ok(file) = my_addr_file {
                    let reader = BufReader::new(file);
                    for line in reader.lines() {
                        if let Ok(addr) = line {
                            let multiaddr: Multiaddr = addr.parse().unwrap();
                            match swarm.dial(multiaddr) {
                                Ok(_) => {}
                                Err(_) => {}
                            }
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(mybehaviour) => match mybehaviour {
                MyBehaviourEvent::Gossipsub(event) => match event {
                    libp2p::gossipsub::Event::Subscribed { topic, .. } => {
                        if topic.to_string() == "client" {
                            match blockchain_db().await {
                                Ok(db) => {
                                    let transactions_coll: Collection<Document> =
                                        db.collection("Transactions");
                                    let pipeline = vec![doc! {
                                        "$match": {
                                            "operationType": "insert"
                                        }
                                    }];
                                    let mut watchin =
                                        transactions_coll.watch(pipeline, None).await.unwrap();

                                    loop {
                                        if let Some(change) = watchin.next().await {
                                            match change {
                                                Ok(data) => {
                                                    if let Some(doc) = data.full_document {
                                                        let transaction: Transaction =
                                                            from_document(doc).unwrap();
                                                        let str_trx =
                                                            serde_json::to_string(&transaction)
                                                                .unwrap();
                                                        let send_transaction = swarm
                                                            .behaviour_mut()
                                                            .gossipsub
                                                            .publish(
                                                                clients_topic.clone(),
                                                                str_trx.as_bytes(),
                                                            );
                                                        match send_transaction {
                                                            Ok(_) => {
                                                                //insert transaction to db
                                                                handle_transactions(
                                                                    str_trx,
                                                                    db.clone(),
                                                                )
                                                                .await;
                                                            }
                                                            Err(_) => {
                                                                write_log("Sending Trx to Client Error! check-mongo-changes(line 86)");
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(_) => {}
                                            }
                                        }
                                    }
                                }
                                Err(_) => {
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                },
                MyBehaviourEvent::ReqRes(event) => match event {
                    Event::Message { message, .. } => match message {
                        // Message::Response { _response, .. } => {}
                        _ => {}
                    },
                    _ => {}
                },
            },
            _ => {}
        }
    }
}
