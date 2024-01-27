use libp2p::{swarm::NetworkBehaviour, PeerId, request_response::{cbor, ResponseChannel}};
use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};
use sp_core::ecdsa::Signature;
use serde_with::{serde_as, DisplayFromStr};

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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct BlockSign {
    pub public: Vec<u8>,
    pub signature: Vec<Signature>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Body {
    pub coinbase: CoinbaseTransaction,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub body: Body,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct TxInput {
    pub input_hash: String,
    pub input_data: InputData,
    pub signatures: Vec<Signature>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct InputData {
    pub number: u8,
    pub utxos: Vec<UtxoData>,
    pub script: TransactionScript,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct TxOutput {
    pub output_hash: String,
    pub output_data: OutputData,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct OutputData {
    pub number: u8,
    pub utxos: Vec<OutputUtxo>,
    pub sigenr_public_keys: Vec<sp_core::ecdsa::Public>,
    #[serde_as(as = "DisplayFromStr")]
    pub fee: Decimal,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct OutputUtxo {
    pub hash: String,
    pub output_unspent: OutputUnspent,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct OutputUnspent {
    pub public_key: String,
    #[serde_as(as = "DisplayFromStr")]
    pub unspent: Decimal,
    pub rnum: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct BlockHeader {
    pub blockhash: String,
    pub prevhash: String,
    pub number: i64,
    pub validator: String,
    pub validator_blocks_number: u64,
    pub merkel_root: String,
    pub block_signature: BlockSign,
    pub generate_time: String
}

#[serde_as]
//a UTXO structure model
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct UtxoData {
    pub transaction_hash: String,
    #[serde_as(as = "DisplayFromStr")]
    pub unspent: Decimal,
    pub output_hash: String,
    pub block_number: i64
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum TransactionScript {
    SingleSig,
    MultiSig,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Transaction {
    pub tx_hash: String,
    pub input: TxInput,
    pub output: TxOutput,
    #[serde_as(as = "DisplayFromStr")]
    pub value: Decimal,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CoinbaseTransaction {
    pub tx_hash: String,
    pub coinbase_data: CoinbaseData,
    pub output: CoinbaseOutput,
    #[serde_as(as = "DisplayFromStr")]
    pub value: Decimal,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CoinbaseData {
    pub block_len: usize,
    pub merkel_root: String,
    #[serde_as(as = "DisplayFromStr")]
    pub reward: Decimal,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CoinbaseOutput {
    pub utxos: Vec<OutputUtxo>,
    pub number: u8,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct GossipMessage {
    pub block: Block,
    pub next_leader: String,
}