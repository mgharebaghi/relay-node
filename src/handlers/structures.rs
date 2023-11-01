use libp2p::{swarm::NetworkBehaviour, PeerId, request_response::{cbor, ResponseChannel}};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Req {
    pub req: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Res {
    pub res: String,
}

#[derive(NetworkBehaviour)]
pub struct CustomBehav {
    pub gossipsub: libp2p::gossipsub::Behaviour,
    pub req_res: cbor::Behaviour<Req, Res>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqForReq {
    pub peer: Vec<PeerId>,
    pub req: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResForReq {
    pub peer: Vec<PeerId>,
    pub res: Res,
}

#[derive(Debug)]
pub struct Channels {
    pub peer: PeerId,
    pub channel: ResponseChannel<Res>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImSync {
    pub peerid: PeerId,
    pub msg: String,
    pub public_key: sp_core::ecdsa::Public,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutNode {
    pub peer_id: PeerId,
}
