use std::{time::Duration, fs::File, io::{BufReader, BufRead}};

use axum::{extract, Json};
use libp2p::{identity::Keypair, PeerId, request_response::{cbor, ProtocolSupport, Config, Message}, StreamProtocol, SwarmBuilder, Multiaddr, futures::StreamExt, swarm::SwarmEvent};

use crate::handlers::structures::{Req, ReqForReq, Res, ResForReq, UTXO};

use super::server::ReqForUtxo;

pub async fn handle_utxo(extract::Json(utxo_req): extract::Json<ReqForUtxo>) -> Json<UTXO> {
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
        match swarm.select_next_some().await {
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