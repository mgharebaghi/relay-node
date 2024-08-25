use libp2p::PeerId;
use mongodb::Database;
use serde::{Deserialize, Serialize};
use sp_core::ed25519::Public;

use crate::relay::tools::waiting::Waiting;

#[derive(Debug, Serialize, Deserialize)]
pub struct Validator {
    pub peerid: PeerId,
    pub relay: PeerId,
    pub wallet: Public,
    pub waiting: u64,
}

impl Validator {
    pub async fn new<'a>(
        db: &'a Database,
        peerid: PeerId,
        relay: PeerId,
        wallet: Public,
    ) -> Result<Self, &'a str> {
        //make waiting number to return new validator with its waiting
        match Waiting::new(db).await {
            Ok(waiting) => Ok(Self {
                peerid,
                relay,
                wallet,
                waiting,
            }),
            Err(e) => Err(e),
        }
    }
}
