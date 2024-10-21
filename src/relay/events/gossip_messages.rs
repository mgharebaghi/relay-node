use libp2p::{PeerId, Swarm};
use mongodb::{bson::{doc, Document}, Collection, Database};
use serde::{Deserialize, Serialize};

use crate::relay::{
    practical::{
        block::{block::Block, message::BlockMessage},
        leader::Leader,
        reciept::Reciept,
        swarm::CentichainBehaviour,
        transaction::Transaction,
    },
    tools::{
        create_log::write_log, syncer::{Sync, VSync}, wrongdoer::WrongDoer
    },
};

use super::connections::ConnectionsHandler;

// Enum representing different types of gossip messages that can be handled
#[derive(Debug, Serialize, Deserialize)]
pub enum GossipMessages {
    BlockMessage(BlockMessage),
    Transaction(Transaction),
    SyncMessage(VSync),
    LeaderVote(PeerId),
    Outnode(PeerId),
}

impl GossipMessages {
    // Main handler for processing different types of gossip messages
    pub async fn handle<'a>(
        message: Vec<u8>,
        propagation_source: PeerId,
        db: &'a Database,
        swarm: &mut Swarm<CentichainBehaviour>,
        connections_handler: &mut ConnectionsHandler,
        leader: &mut Leader,
        sync_state: &Sync,
        recvied_blocks: &mut Vec<BlockMessage>,
        last_block: &mut Vec<Block>,
    ) -> Result<(), &'a str> {
        // Attempt to convert the message bytes to a UTF-8 string
        if let Ok(str_message) = String::from_utf8(message) {
            // Try to deserialize the string into a GossipMessages enum
            if let Ok(gossip_message) = serde_json::from_str::<Self>(&str_message) {
                match gossip_message {
                    // Handle block messages
                    GossipMessages::BlockMessage(block_message) => {
                        write_log("Block message received");
                        // Process the block message
                        block_message
                            .handle(
                                swarm,
                                db,
                                recvied_blocks,
                                sync_state,
                                last_block,
                                leader,
                                connections_handler,
                            )
                            .await
                    }

                    // Handle transactions
                    GossipMessages::Transaction(transaction) => {
                        // Check if transaction hash is not in the receipts collection
                        let receipts_coll: Collection<Document> = db.collection("receipts");
                        let filter = doc! {"hash": &transaction.hash};
                        if let Ok(None) = receipts_coll.find_one(filter).await {
                            // Validate the transaction if not found in receipts
                            match transaction.validate(db).await {
                                Ok(trx) => {
                                    // Insert validated transaction into the database
                                    match trx.insertion(db, leader, connections_handler, swarm).await {
                                        Ok(_) => Reciept::insertion(None, Some(trx), None, db).await,
                                        Err(e) => Err(e),
                                    }
                                }
                                Err(_) => {
                                    // Remove the connection if transaction is invalid
                                    connections_handler
                                        .remove(db, propagation_source, swarm)
                                        .await
                                }
                            }
                        } else {
                            // Transaction hash already exists in receipts, no need to validate
                            Ok(())
                        }
                    }

                    // Handle sync messages
                    GossipMessages::SyncMessage(vsync) => {
                        match sync_state {
                            Sync::Synced => vsync.handle(db, leader).await, // Add new validator to validators document if it was a correct message
                            _ => Ok(()),
                        }
                    }

                    // Handle leader votes
                    GossipMessages::LeaderVote(vote) => match sync_state {
                        Sync::Synced => {
                            if leader.in_check {
                                // Process the vote if the leader is being checked
                                leader.check_votes(db, vote).await
                            } else {
                                Ok(())
                            }
                        }
                        Sync::NotSynced => Ok(()),
                    },

                    // Handle outnode messages
                    GossipMessages::Outnode(peerid) => {
                        if peerid == leader.peerid.unwrap() {
                            // Start voting process if the outnode is the current leader
                            leader.start_voting(db, connections_handler, swarm).await
                        } else {
                            // Remove the wrongdoer from the database
                            match WrongDoer::remove(db, peerid).await {
                                Ok(_) => Ok(()),
                                Err(e) => Err(e),
                            }
                        }
                    }
                }
            } else {
                // Return Ok if the message couldn't be deserialized
                Ok(())
            }
        } else {
            // Return Ok if the message couldn't be converted to a string
            Ok(())
        }
    }
}
