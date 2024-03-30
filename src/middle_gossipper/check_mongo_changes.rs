use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use futures::StreamExt;
use libp2p::{
    swarm::SwarmEvent,
    Multiaddr, Swarm,
};
use mongodb::{
    bson::{from_document, Document},
    Collection,
};

use crate::handlers::{
    create_log::write_log,
    db_connection::blockchain_db,
    structures::{Req, Transaction},
};

use super::middlegossiper_swarm::{MyBehaviour, MyBehaviourEvent};

pub async fn checker(swarm: &mut Swarm<MyBehaviour>) {
    write_log("in middle gossipper");
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                write_log("middle gossiper connection stablished");
                match blockchain_db().await {
                    Ok(db) => {
                        let transactions_coll: Collection<Document> = db.collection("Transactions");
                        let mut watchin = transactions_coll.watch(None, None).await.unwrap();
                        loop {
                            println!("in loop of subscriber");
                            if let Some(change) = watchin.next().await {
                                write_log("new transaction find!");
                                match change {
                                    Ok(data) => {
                                        let doc = data.full_document.unwrap();
                                        let transaction: Transaction = from_document(doc).unwrap();
                                        let str_trx = serde_json::to_string(&transaction).unwrap();
                                        let request = Req { req: str_trx };
                                        swarm
                                            .behaviour_mut()
                                            .req_res
                                            .send_request(&peer_id, request);
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
                    _ => {}
                },
                MyBehaviourEvent::ReqRes(event) => match event {
                    _ => {}
                },
            },
            _ => {}
        }
    }
}
