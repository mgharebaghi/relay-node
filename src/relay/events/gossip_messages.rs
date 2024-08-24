use serde::Deserialize;

use crate::relay::{practical::{block::message::BlockMessage, transaction::Transaction}, tools::syncer::VSync};

#[derive(Debug, Deserialize)]
pub enum GossipMessages {
    BlockMessage(BlockMessage),
    Transaction(Transaction),
    SyncMessage(VSync),
}

impl GossipMessages {
    pub fn check_message(message: Vec<u8>) {
        if let Ok(str_message) = String::from_utf8(message) {
            if let Ok(gossip_message) = serde_json::from_str::<Self>(&str_message) {
                match gossip_message {
                    GossipMessages::BlockMessage(block) => {
                        println!("{:?}", block)
                    }
                    GossipMessages::Transaction(transaction) => {
                        println!("{:?}", transaction);
                    }
                    GossipMessages::SyncMessage(vsync) => {
                        println!("{:?}", vsync)
                    }
                }
            }
        }
    }
}
