use libp2p::{
    futures::StreamExt,
    gossipsub::{Behaviour, IdentTopic},
    identity::Keypair,
    request_response::{cbor, Config, Message, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use sp_core::ecdsa::{Public, Signature};
use std::{
    fs::File,
    io::{BufRead, BufReader},
};
use std::{net::SocketAddr, time::Duration};

use axum::{extract, http::Method, routing::post, Json, Router};
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

pub async fn handle_requests() {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers(AllowHeaders::any());
    let app: Router = Router::new()
        .route("/tx", post(handle_transaction))
        .route("/utxo", post(handle_utxo))
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

#[derive(NetworkBehaviour)]
struct TxBehaviour {
    gossipsub: libp2p::gossipsub::Behaviour,
}

async fn handle_transaction(extract::Json(transaction): extract::Json<Transaction>) -> String {
    let tx_topic = IdentTopic::new("transaction");
    let node_topic = IdentTopic::new("client");
    //generate peer keys and peer id for network
    let keypair = Keypair::generate_ecdsa();
    // let local_peer_id = PeerId::from(keypair.public());

    //gossip protocol config
    let privacy = libp2p::gossipsub::MessageAuthenticity::Signed(keypair.clone());
    let gossip_cfg_builder = libp2p::gossipsub::ConfigBuilder::default();
    let gossip_cfg = libp2p::gossipsub::ConfigBuilder::build(&gossip_cfg_builder).unwrap();
    let gossipsub: Behaviour = libp2p::gossipsub::Behaviour::new(privacy, gossip_cfg).unwrap();
    let mut behaviour = TxBehaviour { gossipsub };
    behaviour.gossipsub.subscribe(&tx_topic).unwrap();

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

    let str_transaction = serde_json::to_string(&transaction).unwrap();

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::ConnectionEstablished { .. } => {
                println!("connection stablished");
            }
            SwarmEvent::Behaviour(gossipevent) => match gossipevent {
                TxBehaviourEvent::Gossipsub(gossipsub) => match gossipsub {
                    libp2p::gossipsub::Event::Subscribed { .. } => {
                        println!("subscribed");
                        let send_message = swarm
                            .behaviour_mut()
                            .gossipsub
                            .publish(node_topic.clone(), str_transaction.as_bytes());

                        match send_message {
                            Ok(_) => {
                                println!("{:?}", transaction);
                                return "Your transaction sent.".to_string();
                            }
                            Err(_) => {}
                        }
                    }
                    _ => {}
                },
            },
            _ => {}
        }
    }
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
                        return handle_response(response);
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}

fn handle_response(response: Res) -> Json<UTXO> {
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
