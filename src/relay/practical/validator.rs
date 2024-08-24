use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sp_core::ed25519::Public;

#[derive(Debug, Serialize, Deserialize)]
pub struct Validator {
    pub peerid: PeerId,
    pub relay: PeerId,
    pub wallet: Public,
    pub waiting: u64
}