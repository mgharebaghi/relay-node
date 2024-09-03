use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sp_core::ed25519::{Public, Signature};

// Define the structure of a block's header with signature.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Header {
    pub number: u64,
    pub hash: String,
    pub previous: String,
    pub validator: PeerId,
    pub relay_id: PeerId,
    merkel: String,
    pub signature: Sign,
    date: String,
}

// Define the structure of a signature, including the signature itself and the public key for verification.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Sign {
    pub signatgure: Signature,
    pub key: Public,
}