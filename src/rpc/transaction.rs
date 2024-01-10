use std::{time::Duration, fs::File, io::{BufReader, BufRead}};

use axum::{extract, Json};
use libp2p::{identity::Keypair, request_response::{cbor, ProtocolSupport, Config, Event, Message}, StreamProtocol, SwarmBuilder, Multiaddr, futures::StreamExt, swarm::SwarmEvent};

use super::{Transaction, server::{TxRes, Req, Res}};

pub async fn handle_transaction(extract::Json(transaction): extract::Json<Transaction>) -> Json<TxRes> {
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
                        if response.res == "Your transaction sent.".to_string() {
                            let tx_res = TxRes {
                                hash: transaction.tx_hash,
                                status: "sent".to_string(),
                                description: "Wait for submit".to_string(),
                            };
                            return Json(tx_res);
                        } else {
                            let tx_res = TxRes {
                                hash: transaction.tx_hash,
                                status: "Error".to_string(),
                                description: "Wrong transaction or no node".to_string(),
                            };
                            return Json(tx_res);
                        }
                    }
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}