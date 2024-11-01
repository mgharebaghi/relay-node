use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::net::SocketAddr;
use tower::limit::ConcurrencyLimitLayer;

use axum::{http::Method, routing::{get, post}, Router};
use tower_http::{
    cors::{AllowHeaders, Any, CorsLayer},
    services::ServeDir,
};

use crate::relay::{practical::block::block::Block, tools::create_log::write_log};

use super::{
    block::handle_block,
    one_utxo::a_utxo,
    reciept::{handle_reciept, handle_user_reciepts, ws_reciept},
    transaction::handle_transaction,
    utxo::{handle_utxo, handle_utxo_ws},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqForUtxo {
    pub wallet: String,
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
    pub status: String,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TxRes {
    pub hash: String,
    pub status: String,
    pub description: String,
}

pub struct Rpc;

impl Rpc {
    pub async fn handle_requests() {
        let cors = CorsLayer::new()
            .allow_methods([Method::GET, Method::POST])
            .allow_origin(Any)
            .allow_headers(AllowHeaders::any());

        let ws_layer = tower_http::cors::CorsLayer::permissive()
            .allow_headers([
                axum::http::HeaderName::from_static("upgrade"),
                axum::http::HeaderName::from_static("sec-websocket-key"),
                axum::http::HeaderName::from_static("sec-websocket-protocol"),
                axum::http::HeaderName::from_static("sec-websocket-version"),
            ]);

        let app: Router = Router::new()
            .route("/trx", post(handle_transaction))
            .route("/utxo", post(handle_utxo))
            .route("/reciept", post(handle_reciept))
            .route("/urec", post(handle_user_reciepts))
            .route("/block", post(handle_block))
            .route("/autxo", post(a_utxo))
            .route("/reciept/ws", get(|ws| ws_reciept(ws)))
            .route("/utxo/ws", get(|ws| handle_utxo_ws(ws)))
            .layer(cors)
            .layer(ws_layer)
            .layer(ConcurrencyLimitLayer::new(100))
            .nest_service("/blockchain", ServeDir::new("/home"));

        // let config = RustlsConfig::from_pem_file("/etc/cert.pem", "/etc/key.pem").await.unwrap();

        let addr = SocketAddr::from(([0, 0, 0, 0], 33369));

        match axum_server::bind(addr).serve(app.into_make_service()).await {
            Ok(_) => {}
            Err(e) => write_log(&format!("error from RPC server:\n{}", e)),
        }
    }
}
