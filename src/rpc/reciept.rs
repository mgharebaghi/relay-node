use std::{
    fs::File,
    io::{BufRead, BufReader},
    str::FromStr,
    time::Duration,
};

use axum::{extract, Json};
use libp2p::{
    futures::StreamExt,
    identity::Keypair,
    request_response::{cbor, Config, Event, Message, ProtocolSupport},
    swarm::SwarmEvent,
    Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use rust_decimal::Decimal;

use crate::handlers::structures::{Req, ReqForReq, Res, ResForReq};

use super::server::{RcptReq, RcptRes, Reciept, TxReq};

pub async fn handle_reciept(extract::Json(tx_req): extract::Json<TxReq>) -> Json<Reciept> {
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
                fee: Decimal::from_str("0.0").unwrap(),
                status: "Error".to_string(),
                description: "Transaction not found!".to_string(),
                date: "".to_string(),
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
            fee: Decimal::from_str("0.0").unwrap(),
            status: "Error".to_string(),
            description: "Transaction not found!".to_string(),
            date: "".to_string(),
        };
        return Json(reciept);
    }
}

pub async fn handle_user_reciepts(
    extract::Json(rcpt_req): extract::Json<RcptReq>,
) -> Json<RcptRes> {
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
