use libp2p::{
    futures::StreamExt,
    identity::Keypair,
    request_response::{cbor, Config, Event, Message, ProtocolSupport},
    swarm::SwarmEvent,
    Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use sp_core::ecdsa::{Public, Signature};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    str::FromStr,
};
use std::{net::SocketAddr, time::Duration};

use axum::{extract, http::Method, routing::get, routing::post, Json, Router};
use tower_http::cors::{AllowHeaders, Any, CorsLayer};

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
    peer: Vec<PeerId>,
    req: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReqForUtxo {
    public_key: String,
    request: String,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
struct Reciept {
    pub block_number: Option<i64>,
    pub hash: String,
    pub from: String,
    pub to: String,
    #[serde_as(as = "DisplayFromStr")]
    pub value: Decimal,
    pub satatus: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TxReq {
    tx_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RcptReq {
    public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RcptRes {
    all: Vec<Reciept>,
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
        .layer(cors);
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

async fn handle_transaction(extract::Json(transaction): extract::Json<Transaction>) -> String {
    let keypair = Keypair::generate_ecdsa();
    // let peerid = PeerId::from(keypair.public());
    let behaviour = cbor::Behaviour::<Req, Res>::new(
        [(StreamProtocol::new("/mg/1.0"), ProtocolSupport::Full)],
        Config::default(),
    );

    //config swarm
    let swarm_config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(10 * 60));
    let mut swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            Default::default(),
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_dns()
        .unwrap()
        .with_websocket(
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .await
        .unwrap()
        .with_behaviour(|_key| behaviour)
        .unwrap()
        .with_swarm_config(|_conf| swarm_config)
        .build();

    let listener: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
    swarm.listen_on(listener).unwrap();

    let mut dial_addr = String::new();

    let address_file = File::open("/etc/myaddress.dat").unwrap();
    let reader = BufReader::new(address_file);
    for i in reader.lines() {
        let addr = i.unwrap();
        if addr.trim().len() > 0 {
            dial_addr.push_str(&addr);
            break;
        }
    }

    let dial_multiaddr: Multiaddr = dial_addr.parse().unwrap();
    swarm.dial(dial_multiaddr).unwrap();

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                let request = Req {
                    req: serde_json::to_string(&transaction).unwrap(),
                };
                swarm.behaviour_mut().send_request(&peer_id, request);
            }
            SwarmEvent::Behaviour(event) => match event {
                Event::Message { message, .. } => match message {
                    Message::Response { response, .. } => {
                        return response.res;
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}

async fn handle_utxo(extract::Json(utxo_req): extract::Json<ReqForUtxo>) -> Json<UTXO> {
    let keypair = Keypair::generate_ecdsa();
    let peerid = PeerId::from(keypair.public());
    let behaviour = cbor::Behaviour::<Req, Res>::new(
        [(StreamProtocol::new("/mg/1.0"), ProtocolSupport::Full)],
        Config::default(),
    );

    //config swarm
    let swarm_config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(10 * 60));
    let mut swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            Default::default(),
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_dns()
        .unwrap()
        .with_websocket(
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .await
        .unwrap()
        .with_behaviour(|_key| behaviour)
        .unwrap()
        .with_swarm_config(|_conf| swarm_config)
        .build();

    let listener: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
    swarm.listen_on(listener).unwrap();

    let mut dial_addr = String::new();

    let address_file = File::open("/etc/myaddress.dat").unwrap();
    let reader = BufReader::new(address_file);
    for i in reader.lines() {
        let addr = i.unwrap();
        if addr.trim().len() > 0 {
            dial_addr.push_str(&addr);
            break;
        }
    }

    let dial_multiaddr: Multiaddr = dial_addr.parse().unwrap();
    swarm.dial(dial_multiaddr).unwrap();

    let request = ReqForReq {
        peer: vec![peerid],
        req: serde_json::to_string(&utxo_req).unwrap(),
    };

    loop {
        match swarm.next().await.unwrap() {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                let req = Req {
                    req: serde_json::to_string(&request).unwrap(),
                };
                swarm.behaviour_mut().send_request(&peer_id, req);
            }
            SwarmEvent::Behaviour(req_res) => match req_res {
                libp2p::request_response::Event::Message { message, .. } => match message {
                    Message::Response { response, .. } => {
                        return handle_utxo_response(response);
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}

fn handle_utxo_response(response: Res) -> Json<UTXO> {
    if let Ok(res) = serde_json::from_str::<ResForReq>(&response.res) {
        if let Ok(utxo) = serde_json::from_str::<UTXO>(&res.res.res) {
            return Json(utxo);
        } else {
            let utxo = UTXO {
                public_key: "".to_string(),
                utxos: Vec::new(),
            };
            return Json(utxo);
        }
    } else {
        let utxo = UTXO {
            public_key: "".to_string(),
            utxos: Vec::new(),
        };
        return Json(utxo);
    }
}

async fn handle_reciept(extract::Json(tx_req): extract::Json<TxReq>) -> Json<Reciept> {
    let keypair = Keypair::generate_ecdsa();
    let peerid = PeerId::from(keypair.public());
    let behaviour = cbor::Behaviour::<Req, Res>::new(
        [(StreamProtocol::new("/mg/1.0"), ProtocolSupport::Full)],
        Config::default(),
    );

    //config swarm
    let swarm_config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(10 * 60));
    let mut swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            Default::default(),
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_dns()
        .unwrap()
        .with_websocket(
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .await
        .unwrap()
        .with_behaviour(|_key| behaviour)
        .unwrap()
        .with_swarm_config(|_conf| swarm_config)
        .build();

    let listener: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
    swarm.listen_on(listener).unwrap();

    let mut dial_addr = String::new();

    let address_file = File::open("/etc/myaddress.dat").unwrap();
    let reader = BufReader::new(address_file);
    for i in reader.lines() {
        let addr = i.unwrap();
        if addr.trim().len() > 0 {
            dial_addr.push_str(&addr);
            break;
        }
    }

    let dial_multiaddr: Multiaddr = dial_addr.parse().unwrap();
    swarm.dial(dial_multiaddr).unwrap();

    let request = ReqForReq {
        peer: vec![peerid],
        req: serde_json::to_string(&tx_req).unwrap(),
    };

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                let req = Req {
                    req: serde_json::to_string(&request).unwrap(),
                };
                swarm.behaviour_mut().send_request(&peer_id, req);
            }
            SwarmEvent::Behaviour(req_res) => match req_res {
                Event::Message { message, .. } => match message {
                    Message::Response { response, .. } => {
                        return handle_reciept_response(response, tx_req.tx_hash);
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}

fn handle_reciept_response(response: Res, tx_hash: String) -> Json<Reciept> {
    if let Ok(res) = serde_json::from_str::<ResForReq>(&response.res) {
        if let Ok(reciept) = serde_json::from_str::<Reciept>(&res.res.res) {
            return Json(reciept);
        } else {
            let reciept = Reciept {
                block_number: None,
                hash: tx_hash,
                from: String::new(),
                to: String::new(),
                value: Decimal::from_str("0.0").unwrap(),
                satatus: "Unconfirmed".to_string(),
                description: "Transaction not found!".to_string(),
            };
            return Json(reciept);
        }
    } else {
        let reciept = Reciept {
            block_number: None,
            hash: tx_hash,
            from: String::new(),
            to: String::new(),
            value: Decimal::from_str("0.0").unwrap(),
            satatus: "Unconfirmed".to_string(),
            description: "Transaction not found!".to_string(),
        };
        return Json(reciept);
    }
}

async fn handle_user_reciepts(extract::Json(rcpt_req): extract::Json<RcptReq>) -> Json<RcptRes> {
    let keypair = Keypair::generate_ecdsa();
    let peerid = PeerId::from(keypair.public());
    let behaviour = cbor::Behaviour::<Req, Res>::new(
        [(StreamProtocol::new("/mg/1.0"), ProtocolSupport::Full)],
        Config::default(),
    );

    //config swarm
    let swarm_config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(10 * 60));
    let mut swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            Default::default(),
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_dns()
        .unwrap()
        .with_websocket(
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .await
        .unwrap()
        .with_behaviour(|_key| behaviour)
        .unwrap()
        .with_swarm_config(|_conf| swarm_config)
        .build();

    let listener: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
    swarm.listen_on(listener).unwrap();

    let mut dial_addr = String::new();

    let address_file = File::open("/etc/myaddress.dat").unwrap();
    let reader = BufReader::new(address_file);
    for i in reader.lines() {
        let addr = i.unwrap();
        if addr.trim().len() > 0 {
            dial_addr.push_str(&addr);
            break;
        }
    }

    let dial_multiaddr: Multiaddr = dial_addr.parse().unwrap();
    swarm.dial(dial_multiaddr).unwrap();

    let request = ReqForReq {
        peer: vec![peerid],
        req: serde_json::to_string(&rcpt_req).unwrap(),
    };

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                let req = Req {
                    req: serde_json::to_string(&request).unwrap(),
                };
                swarm.behaviour_mut().send_request(&peer_id, req);
            }
            SwarmEvent::Behaviour(req_res) => match req_res {
                Event::Message { message, .. } => match message {
                    Message::Response { response, .. } => {
                        return handle_user_rcpts_response(response);
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}

fn handle_user_rcpts_response(response: Res) -> Json<RcptRes> {
    if let Ok(res) = serde_json::from_str::<ResForReq>(&response.res) {
        if let Ok(rcpts_res) = serde_json::from_str::<RcptRes>(&res.res.res) {
            return Json(rcpts_res);
        } else {
            let rcpts_res = RcptRes { all: Vec::new() };
            return Json(rcpts_res);
        }
    } else {
        let rcpts_res = RcptRes { all: Vec::new() };
        return Json(rcpts_res);
    }
}

async fn handle_all_reciepts() -> Json<RcptRes> {
    let keypair = Keypair::generate_ecdsa();
    let peerid = PeerId::from(keypair.public());
    let behaviour = cbor::Behaviour::<Req, Res>::new(
        [(StreamProtocol::new("/mg/1.0"), ProtocolSupport::Full)],
        Config::default(),
    );

    //config swarm
    let swarm_config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(10 * 60));
    let mut swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            Default::default(),
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_dns()
        .unwrap()
        .with_websocket(
            (libp2p::tls::Config::new, libp2p::noise::Config::new),
            libp2p::yamux::Config::default,
        )
        .await
        .unwrap()
        .with_behaviour(|_key| behaviour)
        .unwrap()
        .with_swarm_config(|_conf| swarm_config)
        .build();

    let listener: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
    swarm.listen_on(listener).unwrap();

    let mut dial_addr = String::new();

    let address_file = File::open("/etc/myaddress.dat").unwrap();
    let reader = BufReader::new(address_file);
    for i in reader.lines() {
        let addr = i.unwrap();
        if addr.trim().len() > 0 {
            dial_addr.push_str(&addr);
            break;
        }
    }

    let dial_multiaddr: Multiaddr = dial_addr.parse().unwrap();
    swarm.dial(dial_multiaddr).unwrap();

    let request = ReqForReq {
        peer: vec![peerid],
        req: "allrec_req".to_string(),
    };

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                let req = Req {
                    req: serde_json::to_string(&request).unwrap(),
                };
                swarm.behaviour_mut().send_request(&peer_id, req);
            }
            SwarmEvent::Behaviour(req_res) => match req_res {
                Event::Message { message, .. } => match message {
                    Message::Response { response, .. } => {
                        return handle_allrec_response(response);
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}

fn handle_allrec_response(response: Res) -> Json<RcptRes> {
    if let Ok(res) = serde_json::from_str::<ResForReq>(&response.res) {
        if let Ok(rcpts_res) = serde_json::from_str::<RcptRes>(&res.res.res) {
            return Json(rcpts_res);
        } else {
            let rcpts_res = RcptRes { all: Vec::new() };
            return Json(rcpts_res);
        }
    } else {
        let rcpts_res = RcptRes { all: Vec::new() };
        return Json(rcpts_res);
    }
}
