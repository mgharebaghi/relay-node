use libp2p::{PeerId, Swarm};
use mongodb::Database;
use serde::Deserialize;

use crate::relay::{
    practical::{
        block::message::BlockMessage, swarm::CentichainBehaviour, transaction::Transaction,
    },
    tools::{create_log::write_log, syncer::VSync},
};

use super::connections::ConnectionsHandler;

#[derive(Debug, Deserialize)]
pub enum GossipMessages {
    BlockMessage(BlockMessage),
    Transaction(Transaction),
    SyncMessage(VSync),
}

impl GossipMessages {
    pub async fn handle(
        message: Vec<u8>,
        propagation_source: PeerId,
        db: &Database,
        swarm: &mut Swarm<CentichainBehaviour>,
        connections_handler: &mut ConnectionsHandler,
    ) {
        if let Ok(str_message) = String::from_utf8(message) {
            if let Ok(gossip_message) = serde_json::from_str::<Self>(&str_message) {
                match gossip_message {
                    //check block messages
                    GossipMessages::BlockMessage(block_message) => {
                        //at first checks next leader and if that was true then checks block
                        println!("{:?}", block_message)
                    }

                    //check transactions
                    GossipMessages::Transaction(transaction) => {
                        //check transactions
                        match transaction.validate(db).await {
                            Ok(trx) => {
                                //insert transaction to transactions collection
                                match trx.insertion(db).await {
                                    Ok(_) => {}
                                    Err(e) => write_log(e),
                                }
                            }
                            Err(_) => {
                                match connections_handler.remove(db, propagation_source).await {
                                    Ok(_) => {
                                        swarm.disconnect_peer_id(propagation_source).unwrap();
                                    }
                                    Err(e) => write_log(e),
                                }
                            }
                        }
                    }

                    //handle sync messages
                    GossipMessages::SyncMessage(vsync) => {
                        //add sync messages user to validators
                        match vsync.handle(db).await {
                            Ok(_) => {}
                            Err(e) => {
                                write_log(e);
                                std::process::exit(0);
                            }
                        }
                        println!("{:?}", vsync)
                    }
                }
            }
        }
    }
}
