use serde::Deserialize;

use crate::handlers::tools::syncer::VSync;

use super::{block::message::BlockMessage, transaction::Transaction};

#[derive(Debug, Deserialize)]
pub enum GossipMessages {
    BlockMessage(BlockMessage),
    Transaction(Transaction),
    SyncMessage(VSync),
}

impl GossipMessages {
    pub fn check_message(message: GossipMessages) {
        match message {
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
