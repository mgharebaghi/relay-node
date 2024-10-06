use libp2p::{PeerId, Swarm};
use mongodb::{
    bson::{to_document, Document},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use crate::relay::{
    events::connections::ConnectionsHandler,
    practical::{leader::Leader, reciept::Reciept, swarm::CentichainBehaviour},
    tools::{create_log::write_log, syncer::Sync},
};

use super::block::Block;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockMessage {
    pub block: Block,
    pub next_leader: PeerId,
}

impl BlockMessage {
    // Handle received block messages
    pub async fn handle<'a>(
        &self,
        swarm: &mut Swarm<CentichainBehaviour>,
        db: &'a Database,
        recvied_blocks: &mut Vec<Self>,
        sync_state: &Sync,
        last_block: &mut Vec<Block>,
        leader: &mut Leader,
        connections_handler: &mut ConnectionsHandler,
    ) -> Result<(), &'a str> {
        // Check if the current node is the leader
        if self.block.header.validator == leader.peerid.unwrap() {
            match sync_state {
                // If the current relay node is synced, proceed with block validation
                Sync::Synced => match self.block.validation(last_block, db).await {
                    // If the block is valid, insert it into the blockchain
                    Ok(block) => {
                        let collection: Collection<Document> = db.collection("Blocks");
                        let doc = to_document(&block).unwrap();
                        match collection.insert_one(doc).await {
                            // If block insertion succeeds, insert receipts for coinbase output and transactions
                            Ok(_) => {
                                let mut is_err = None;
                                // Insert receipt for coinbase
                                match Reciept::insertion(Some(self.block.header.number), None, Some(&self.block.body.coinbase), db).await {
                                    Ok(_) => {
                                        // Confirm receipts for all transactions in the block
                                        for transaction in self.block.body.transactions.clone() {
                                            match Reciept::confirmation(db, &transaction.hash, &self.block.header.number).await {
                                                Ok(_) => {
                                                    println!("block {:?} inserted successfully", block.header.number);
                                                }
                                                Err(e) => {
                                                    is_err.get_or_insert(e);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {is_err.get_or_insert(e);}
                                }
                                
                                match is_err {
                                    None => {
                                        // Update last block and leader if all receipts are inserted successfully
                                        last_block.clear();
                                        last_block.push(block.clone());
                                        Ok(leader.update(Some(self.next_leader)))
                                    },
                                    Some(e) => Err(e)
                                }
                            }
                            Err(_) => Err("Error while inserting new block to database-(generator/block/message 67)")
                        }
                    }
                    // If block validation fails, remove the validator and start voting for a new leader
                    Err(e) => {
                        match connections_handler.remove(db, self.block.header.validator, swarm).await {
                            Ok(_) => {
                                write_log(e);
                                leader.start_voting(db, connections_handler, swarm).await
                            }
                            Err(e) => Err(e)
                        }
                    }
                },
                // If the current relay node is not synced, store the received block message for later processing
                Sync::NotSynced => Ok(recvied_blocks.push(self.clone())),
            }
        } else {
            println!("leader is wrong: {:?}", leader.peerid.unwrap());
            println!("block leader is: {:?}", self.block.header.validator);
            // If the block is from an unexpected validator, remove it from the network
            connections_handler
                .remove(db, self.block.header.validator, swarm)
                .await
        }
    }
}
