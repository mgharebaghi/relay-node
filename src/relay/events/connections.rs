use libp2p::{PeerId, Swarm};
use mongodb::{
    bson::{doc, Document},
    Collection, Database,
};

use crate::relay::{
    practical::{
        block::{block::Block, message::BlockMessage},
        leader::Leader,
        relay::{DialedRelays, First},
        swarm::CentichainBehaviour,
    },
    tools::{
        create_log::write_log,
        syncer::{Sync, Syncer},
        waiting::Waiting,
        wrongdoer::WrongDoer,
    },
};

use super::addresses::Listeners;

pub struct ConnectionsHandler {
    pub connections: Vec<Connection>,
}

//change when relay gets subscribtion
//if peerid was in connection change its kind based on subscribe topic
#[derive(Debug, PartialEq)]
pub enum Kind {
    Relay,
    Validator,
}

//define new connection
#[derive(Debug, PartialEq)]
pub struct Connection {
    pub peerid: PeerId,
    pub kind: Option<Kind>,
}

impl Connection {
    fn new(peerid: PeerId, kind: Option<Kind>) -> Self {
        Self {
            peerid,
            kind: if kind.is_some() { kind } else { None },
        }
    }

    pub fn update(&mut self, kind: Kind) {
        self.kind.get_or_insert(kind);
    }
}

impl ConnectionsHandler {
    //return new stablished handler with an empty vec for store stablished connection
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    //push new stablished connection's peer id to contact of stablished handler
    fn push_new_connection(&mut self, peerid: PeerId) {
        let new_connection = Connection::new(peerid, None);
        self.connections.push(new_connection)
    }

    //update kind of connection
    pub fn update_connection(&mut self, peerid: PeerId, kind: Kind) {
        match self
            .connections
            .iter()
            .position(|conn| conn.peerid == peerid)
        {
            Some(i) => {
                self.connections[i].update(kind); //change kind of connection if it was in the connections vector
            }
            None => {}
        }
    }

    //update dialed relays and then syncing with the network if relay is not synced
    pub async fn update_and_sync<'a>(
        &mut self,
        dialed_relays: &mut DialedRelays,
        connection_peerid: PeerId,
        db: &'a Database,
        sync_state: &mut Sync,
        recieved_blocks: &mut Vec<BlockMessage>,
        multiaddress: &String,
        peerid: &PeerId,
        last_block: &mut Vec<Block>,
        leader: &mut Leader,
    ) -> Result<(), &'a str> {
        //if connections stablished was in dialed relays then goes to syncing
        //else push to connections vector to check wrongdoers and remove thats' connections
        if dialed_relays
            .relays
            .iter()
            .find(|relay| relay.addr.contains(&connection_peerid.to_string()))
            .is_some()
        {
            write_log(&format!(
                "Connection stablished with this dialed relay: {}",
                connection_peerid
            ));
            //if sync is NotSynced start syncing with the network
            match sync_state {
                Sync::Synced => Ok(()),
                Sync::NotSynced => {
                    write_log("start syncing...");
                    if let Err(e) =
                        Syncer::syncing(db, recieved_blocks, last_block, dialed_relays, leader)
                            .await
                    {
                        write_log(e);
                        Err(e)
                    } else {
                        sync_state.synced(); //if syncing doesn't have problem change sync state to Synced

                        match Listeners::new(&multiaddress.parse().unwrap(), peerid, db).await {
                            Ok(listeners) => match listeners.post().await {
                                Ok(_) => Ok(()),
                                Err(_) => Err(""),
                            },
                            Err(e) => Err(e),
                        }
                    }
                }
            }
        } else {
            write_log(&format!(
                "Connection stablished with: {}",
                connection_peerid
            ));
            Ok(Self::push_new_connection(self, connection_peerid))
        }
    }

    //remove connection from connections ad db if it closed
    pub async fn remove<'a>(
        &mut self,
        db: &'a Database,
        peerid: PeerId,
        swarm: &mut Swarm<CentichainBehaviour>,
    ) -> Result<(), &'a str> {
        //find index of connection in connections for removing from connections
        match self
            .connections
            .iter()
            .position(|conn| conn.peerid == peerid)
        {
            //if peerid was in the connections remove from it else remove as a wrongdoer from validators
            Some(index) => {
                //disconnecting
                match swarm.disconnect_peer_id(peerid) {
                    Ok(_) => {}
                    Err(_) => {}
                }

                //define validator collection for deleting nodes from their
                let collection: Collection<Document> = db.collection("validators");

                //if kind of connection was relay it will be removed from relay collection if there is(if it was relay then its validators will be removed)
                //else it will be removed from validators if there is
                if *self.connections[index].kind.as_ref().unwrap() == Kind::Relay {
                    self.connections.remove(index);
                    let filter = doc! {"relay": peerid.to_string()};
                    //get count of deletes for updating waiting of validators
                    match collection.count_documents(filter.clone()).await {
                        Ok(mut count) => match collection.delete_many(filter).await {
                            Ok(_) => {
                                let mut is_err = None;
                                //update waiting of validators
                                while count > 0 {
                                    match Waiting::update(db, None).await {
                                        Ok(_) => count -= 1,
                                        Err(e) => {
                                            is_err.get_or_insert(e);
                                            break;
                                        }
                                    }
                                }

                                //if there was no any errors return ok else return error
                                match is_err {
                                    None => Ok(()),
                                    Some(e) => Err(e),
                                }
                            }
                            Err(_) => Err(
                                "Deleting validator problem-(handlers/practical/connections 194)",
                            ),
                        },
                        Err(_) => Err(
                            "Get count of documents problem-(handlers/practical/connections 198)",
                        ),
                    }
                } else {
                    self.connections.remove(index);
                    match collection
                        .delete_one(doc! {"peerid": peerid.to_string()})
                        .await
                    {
                        Ok(_) => Waiting::update(db, None).await,
                        Err(_) => {
                            Err("Deleting validator problem-(handlers/practical/connections 209)")
                        }
                    }
                }
            }
            None => match WrongDoer::remove(db, peerid).await {
                Ok(removed) => Ok(write_log(&format!("Wrongdoer removed: {}", removed))),
                Err(e) => Err(e),
            },
        }
    }

    //check relays' connection and if there is no any connections with any relays return true for breaks handle_loop
    pub fn breaker(&self, dialed_relays: &mut DialedRelays) -> bool {
        let mut relays_count = 0;
        //break to dialing if there is no any connections with relays
        if self.connections.len() > 0 {
            for i in 0..self.connections.len() {
                match &self.connections[i].kind {
                    Some(kind) => match kind {
                        Kind::Relay => relays_count += 1,
                        Kind::Validator => {}
                    },
                    None => {}
                }
            }
        }
        if relays_count == 0 && dialed_relays.first == First::No {
            true
        } else {
            false
        }
    }
}
