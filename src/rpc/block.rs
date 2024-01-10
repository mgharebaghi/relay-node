use std::{
    fs::File,
    io::{BufRead, BufReader},
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

use crate::handlers::structures::Block;

use super::server::{ Req, ReqForReq, Res, ResForReq, AllBlocksRes, BlockReq, BlockRes};

pub async fn handle_all_blocks() -> Json<AllBlocksRes> {
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
        req: "allblocks".to_string(),
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
                        return handle_allblocks_response(response);
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}

fn handle_allblocks_response(response: Res) -> Json<AllBlocksRes> {
    if let Ok(res) = serde_json::from_str::<ResForReq>(&response.res) {
        if let Ok(allblocks_res) = serde_json::from_str::<AllBlocksRes>(&res.res.res) {
            return Json(allblocks_res);
        } else {
            let allblocks_res = AllBlocksRes { all: Vec::new() };
            return Json(allblocks_res);
        }
    } else {
        let allblocks_res = AllBlocksRes { all: Vec::new() };
        return Json(allblocks_res);
    }
}

pub async fn handle_block(extract::Json(block_req): extract::Json<BlockReq>) -> Json<BlockRes> {
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
        req: serde_json::to_string(&block_req).unwrap(),
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
                        return handle_block_response(response);
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}

fn handle_block_response(response: Res) -> Json<BlockRes> {
    if let Ok(res) = serde_json::from_str::<ResForReq>(&response.res) {
        if let Ok(block) = serde_json::from_str::<Block>(&res.res.res) {
            let block_res = BlockRes {
                block: Some(block),
                status: "".to_string(),
            };
            return Json(block_res);
        } else {
            let block_res = BlockRes {
                block: None,
                status: "Block not found!".to_string(),
            };
            return Json(block_res);
        }
    } else {
        let block_res = BlockRes {
            block: None,
            status: "Block not found!".to_string(),
        };
        return Json(block_res);
    }
}