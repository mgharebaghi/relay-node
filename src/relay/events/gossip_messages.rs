use libp2p::{PeerId, Swarm};
use mongodb::Database;
use serde::Deserialize;

use crate::relay::{
    practical::{
        block::{block::Block, message::BlockMessage},
        leader::Leader,
        swarm::CentichainBehaviour,
        transaction::Transaction,
    },
    tools::syncer::{Sync, VSync},
};

use super::connections::ConnectionsHandler;

#[derive(Debug, Deserialize)]
pub enum GossipMessages {
    BlockMessage(BlockMessage),
    Transaction(Transaction),
    SyncMessage(VSync),
    LeaderVote(PeerId),
}

impl GossipMessages {
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
        if let Ok(str_message) = String::from_utf8(message) {
            if let Ok(gossip_message) = serde_json::from_str::<Self>(&str_message) {
                match gossip_message {
                    //check block messages
                    GossipMessages::BlockMessage(block_message) => {
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

                    //check transactions
                    GossipMessages::Transaction(transaction) => {
                        //check transactions
                        match transaction.validate(db).await {
                            Ok(trx) => {
                                //insert transaction to transactions collection
                                trx.insertion(db, leader, connections_handler, swarm).await
                            }
                            Err(_) => {
                                connections_handler
                                    .remove(db, propagation_source, swarm)
                                    .await
                            }
                        }
                    }

                    //handle sync messages
                    GossipMessages::SyncMessage(vsync) => {
                        match sync_state {
                            Sync::Synced => vsync.handle(db).await, //add new validator to validators document if it was correct message}
                            _ => Ok(()),
                        }
                    }

                    //handle votes
                    GossipMessages::LeaderVote(vote) => match sync_state {
                        Sync::Synced => {
                            if leader.in_check {
                                leader.check_votes(db, vote).await
                            } else {
                                Ok(())
                            }
                        }
                        Sync::NotSynced => Ok(()),
                    },
                }
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}
