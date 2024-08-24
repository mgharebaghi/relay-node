use libp2p::PeerId;
use serde::{Deserialize, Serialize};

use super::block::Block;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockMessage {
    block: Block,
    next_leader: PeerId
}

