use libp2p::PeerId;
use mongodb::{
    bson::{doc, Document},
    Collection, Database,
};

use crate::handlers::tools::{
    create_log::write_log,
    syncer::{Sync, Syncer},
};

use super::{addresses::Listeners, block::block::Block, relay::DialedRelays};

pub struct ConnectionsHandler {
    pub connections: Vec<Connection>,
}

//change when relay gets subscribtion
//if peerid was in connection change its kind based on subscribe topic
#[derive(Debug, PartialEq, Clone)]
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
        recieved_blocks: &mut Vec<Block>,
        multiaddress: &String,
        peerid: &PeerId,
        last_block: &mut Vec<Block>,
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
                    if let Err(e) = Syncer::syncing(db, recieved_blocks, last_block).await {
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
    pub async fn remove<'a>(&mut self, db: &'a Database, peerid: PeerId) -> Result<(), &'a str> {
        //define validator collection for deleting nodes from their
        let v_collection: Collection<Document> = db.collection("validator");

        //find index of connection in connections for removing from connections
        let index = self
            .connections
            .iter()
            .position(|conn| conn.peerid == peerid)
            .unwrap();

        //if kind of connection was relay it will be removed from relay collection if there is(if it was relay then its validators will be removed)
        //else it will be removed from validators if there is
        if *self.connections[index].kind.as_ref().unwrap() == Kind::Relay {
            self.connections.remove(index);
            match v_collection
                .delete_many(doc! {"relay": peerid.to_string()})
                .await
            {
                Ok(_) => Ok(()),
                Err(_) => Err("Deleting validator problem-(handlers/practical/connections 170)"),
            }
        } else {
            self.connections.remove(index);
            match v_collection
                .delete_one(doc! {"peerid": peerid.to_string()})
                .await
            {
                Ok(_) => Ok(()),
                Err(_) => Err("Deleting validator problem-(handlers/practical/connections 183)"),
            }
        }
    }
}
