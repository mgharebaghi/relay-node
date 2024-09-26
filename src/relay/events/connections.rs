use libp2p::{gossipsub::IdentTopic, PeerId, Swarm};
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
        zipp::Zip,
    },
};

use super::{addresses::Listeners, gossip_messages::GossipMessages};

// Struct to handle and manage connections
#[derive(Debug, Clone)]
pub struct ConnectionsHandler {
    pub connections: Vec<Connection>,
}

// Enum to differentiate between types of connections
#[derive(Debug, PartialEq, Clone)]
pub enum Kind {
    Relay,
    Validator,
}

// Struct representing a single connection
#[derive(Debug, PartialEq, Clone)]
pub struct Connection {
    pub peerid: PeerId,
    pub kind: Option<Kind>,
}

impl Connection {
    // Create a new connection with optional kind
    fn new(peerid: PeerId, kind: Option<Kind>) -> Self {
        Self {
            peerid,
            kind: if kind.is_some() { kind } else { None },
        }
    }

    // Update the kind of the connection if not already set
    pub fn update(&mut self, kind: Kind) {
        self.kind.get_or_insert(kind);
    }
}

impl ConnectionsHandler {
    // Initialize a new ConnectionsHandler with an empty vector of connections
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    // Add a new connection to the handler with the given PeerId
    fn push_new_connection(&mut self, peerid: PeerId) {
        let new_connection = Connection::new(peerid, None);
        self.connections.push(new_connection)
    }

    // Update the kind of an existing connection identified by PeerId
    pub fn update_connection(&mut self, peerid: PeerId, kind: Kind) {
        match self
            .connections
            .iter()
            .position(|conn| conn.peerid == peerid)
        {
            Some(i) => {
                self.connections[i].update(kind); // Update kind if connection exists
            }
            None => {}
        }
    }

    // Update dialed relays, synchronize with the network if not already synced, and handle new connections
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
        // Check if the connection is with a dialed relay
        if dialed_relays
            .relays
            .iter()
            .find(|relay| relay.addr.contains(&connection_peerid.to_string()))
            .is_some()
        {
            // Log successful connection with dialed relay
            write_log(&format!(
                "Connection established with this dialed relay: {}",
                connection_peerid
            ));
            // Synchronize if not already synced
            match sync_state {
                Sync::Synced => Ok(()),
                Sync::NotSynced => {
                    // Log start of syncing process
                    write_log("start syncing...");
                    if let Err(e) =
                        Syncer::syncing(db, recieved_blocks, last_block, dialed_relays, leader)
                            .await
                    {
                        // Log syncing error
                        write_log(e);
                        Err(e)
                    } else {
                        sync_state.synced(); // Update sync state if successful

                        // Post new listeners after successful sync
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
            // Log connection with non-relay peer
            write_log(&format!(
                "Connection established with: {}",
                connection_peerid
            ));
            // Create a zip of the database for new connections
            match Zip::maker() {
                Ok(_) => Ok(Self::push_new_connection(self, connection_peerid)),
                Err(e) => Err(e),
            }
        }
    }

    // Remove a connection from the handler and database, and handle associated cleanup
    pub async fn remove<'a>(
        &mut self,
        db: &'a Database,
        peerid: PeerId,
        swarm: &mut Swarm<CentichainBehaviour>,
    ) -> Result<(), &'a str> {
        match self
            .connections
            .iter()
            .position(|conn| conn.peerid == peerid)
        {
            Some(index) => {
                // Attempt to disconnect the peer from the swarm
                match swarm.disconnect_peer_id(peerid) {
                    Ok(_) => {}
                    Err(_) => {}
                }

                let collection: Collection<Document> = db.collection("validators");

                // Handle removal based on connection kind (Relay or Validator)
                if self.connections[index].clone().kind.is_some()
                    && self.connections[index].clone().kind.unwrap() == Kind::Relay
                {
                    self.connections.remove(index);
                    let filter = doc! {"relay": peerid.to_string()};
                    // Remove associated validators and update waiting times
                    match collection.count_documents(filter.clone()).await {
                        Ok(mut count) => match collection.delete_many(filter).await {
                            Ok(_) => {
                                let mut is_err = None;
                                while count > 0 {
                                    match Waiting::update(db, None).await {
                                        Ok(_) => count -= 1,
                                        Err(e) => {
                                            is_err.get_or_insert(e);
                                            break;
                                        }
                                    }
                                }
                                match is_err {
                                    None => {
                                        // Publish outnode message to gossipsub
                                        let gossip_message = GossipMessages::Outnode(peerid);
                                        let str_gossip_message =
                                            serde_json::to_string(&gossip_message).unwrap();
                                        if self.connections.len() > 0 {
                                            match swarm.behaviour_mut().gossipsub.publish(IdentTopic::new("validator"), str_gossip_message.as_bytes()) {
                                                Ok(_) => Ok(()),
                                                Err(_) => Err("Failed to publish outnode message-(handlers/practical/connections.rs 198)")
                                            }
                                        } else {
                                            Ok(())
                                        }
                                    }
                                    Some(e) => Err(e),
                                }
                            }
                            Err(_) => Err(
                                "Deleting validator problem-(handlers/practical/connections 205)",
                            ),
                        },
                        Err(_) => Err(
                            "Get count of documents problem-(handlers/practical/connections 209)",
                        ),
                    }
                } else {
                    // Remove connection and associated validator document
                    self.connections.remove(index);
                    println!("connection removed: {}", peerid);
                    println!("connections count:  {}", self.connections.len());
                    println!("connections: {:?}", self.connections);
                    match collection
                        .delete_one(doc! {"peerid": peerid.to_string()})
                        .await
                    {
                        Ok(_) => {
                            // Publish outnode message to gossipsub if connections remain
                            let gossip_message = GossipMessages::Outnode(peerid);
                            let str_gossip_message =
                                serde_json::to_string(&gossip_message).unwrap();
                            if self.connections.len() > 0 {
                                match swarm.behaviour_mut().gossipsub.publish(IdentTopic::new("validator"), str_gossip_message.as_bytes()) {
                                    Ok(_) => Waiting::update(db, None).await,
                                    Err(_) => Err("Failed to publish outnode message-(handlers/practical/connections.rs 224)")
                                }
                            } else {
                                Waiting::update(db, None).await
                            }
                        }
                        Err(_) => {
                            Err("Deleting validator problem-(handlers/practical/connections 228)")
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

    // Check if there are any relay connections and decide whether to break the handle loop
    pub fn breaker(&self, dialed_relays: &mut DialedRelays) -> bool {
        let mut relays_count = 0;
        // Count the number of relay connections
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
        // Return true if no relay connections and not the first dialed relay
        relays_count == 0 && dialed_relays.first == First::No
    }
}
