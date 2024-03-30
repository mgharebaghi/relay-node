use futures::StreamExt;
use libp2p::{
    gossipsub::{Event, IdentTopic},
    swarm::SwarmEvent,
    Swarm,
};
use mongodb::{
    bson::{from_document, Document},
    Collection,
};

use crate::handlers::{create_log::write_log, db_connection::blockchain_db, structures::Transaction};

use super::middlegossiper_swarm::{MyBehaviour, MyBehaviourEvent};

pub async fn checker(swarm: &mut Swarm<MyBehaviour>) {
    write_log("in middle gossipper");
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(mybehaviour) => match mybehaviour {
                MyBehaviourEvent::Gossipsub(event) => match event {
                    Event::Subscribed { .. } => match blockchain_db().await {
                        Ok(db) => {
                            let transactions_coll: Collection<Document> =
                                db.collection("Transactions");
                            let mut watchin = transactions_coll.watch(None, None).await.unwrap();
                            loop {
                                if let Some(change) = watchin.next().await {
                                    match change {
                                        Ok(data) => {
                                            let doc = data.full_document.unwrap();
                                            let transaction: Transaction =
                                                from_document(doc).unwrap();
                                            let str_trx =
                                                serde_json::to_string(&transaction).unwrap();
                                            match swarm.behaviour_mut().gossipsub.publish(
                                                IdentTopic::new("client"),
                                                str_trx.as_bytes(),
                                            ) {
                                                Ok(_) => {}
                                                Err(_) => {}
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
                    },
                    _ => {}
                },
            },
            _ => {}
        }
    }
}
