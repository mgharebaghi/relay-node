use libp2p::PeerId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use sp_core::ecdsa::{Public, Signature};
use std::net::SocketAddr;
use tower::limit::ConcurrencyLimitLayer;

use axum::{http::Method, routing::get, routing::post, Router};
use tower_http::cors::{AllowHeaders, Any, CorsLayer};

use crate::handlers::structures::Block;

use super::{
    block::{handle_all_blocks, handle_block},
    reciept::{handle_all_reciepts, handle_reciept, handle_user_reciepts},
    transaction::handle_transaction,
    utxo::handle_utxo,
};

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

#[serde_as]
//a UTXO structure model
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct UtxoData {
    pub transaction_hash: String,
    #[serde_as(as = "DisplayFromStr")]
    pub unspent: Decimal,
    pub output_hash: String,
    pub block_number: i64,
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
    pub sigenr_public_keys: Vec<Public>,
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
pub struct UTXO {
    pub public_key: String,
    pub utxos: Vec<UtxoData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Req {
    pub req: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Res {
    pub res: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResForReq {
    pub peer: Vec<PeerId>,
    pub res: Res,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqForReq {
    pub peer: Vec<PeerId>,
    pub req: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqForUtxo {
    pub public_key: String,
    pub request: String,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Reciept {
    pub block_number: Option<i64>,
    pub hash: String,
    pub from: String,
    pub to: String,
    #[serde_as(as = "DisplayFromStr")]
    pub value: Decimal,
    pub fee: Decimal,
    pub status: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxReq {
    pub tx_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RcptReq {
    pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RcptRes {
    pub all: Vec<Reciept>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AllBlocksRes {
    pub all: Vec<Block>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockReq {
    pub block_number: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockRes {
    pub block: Option<Block>,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxRes {
    pub hash: String,
    pub status: String,
    pub description: String,
}

pub async fn handle_requests() {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers(AllowHeaders::any());
    let app: Router = Router::new()
        .route("/tx", post(handle_transaction))
        .route("/utxo", post(handle_utxo))
        .route("/reciept", post(handle_reciept))
        .route("/urec", post(handle_user_reciepts))
        .route("/allrec", get(handle_all_reciepts))
        .route("/allblocks", get(handle_all_blocks))
        .route("/block", post(handle_block))
        .layer(cors)
        .layer(ConcurrencyLimitLayer::new(100));
    let addr = SocketAddr::from(([0, 0, 0, 0], 3390));

    if let Some(ip) = public_ip::addr().await {
        let full_address = format!("http://{}:3390", ip);
        let client = reqwest::Client::new();
        let res = client
            .post("https://centichain.org/api/rpc")
            .body(full_address)
            .send()
            .await;
        match res {
            Ok(_) => println!("Your address sent."),
            Err(_) => println!("problem to send address!"),
        }
        println!("your public ip: {}", ip);
    } else {
        println!("You dont have public ip, listener: {}", addr);
    }

    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
