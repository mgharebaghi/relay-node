use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::net::SocketAddr;
use tower::limit::ConcurrencyLimitLayer;

use axum::{http::Method, routing::get, routing::post, Router};
use tower_http::{
    cors::{AllowHeaders, Any, CorsLayer},
    services::ServeDir,
};

use crate::handlers::structures::Block;

use super::{
    block::handle_block,
    reciept::{handle_reciept, handle_user_reciepts},
    sse::{block_sse, trx_sse},
    transaction::handle_transaction,
    utxo::handle_utxo,
};

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
    pub date: String,
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
        .route("/trx", post(handle_transaction))
        .route("/utxo", post(handle_utxo))
        .route("/reciept", post(handle_reciept))
        .route("/urec", post(handle_user_reciepts))
        .route("/block", post(handle_block))
        .route("/trxsse", get(trx_sse))
        .route("/blocksse", get(block_sse))
        .layer(cors)
        .layer(ConcurrencyLimitLayer::new(100))
        .nest_service(
            "/blockchain",
            ServeDir::new("/etc/dump/Blockchain"),
        );
    let addr = SocketAddr::from(([0, 0, 0, 0], 3369));

    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
