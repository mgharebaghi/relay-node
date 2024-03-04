use std::{
    convert::Infallible,
    fs::File,
    io::{BufRead, BufReader},
    time::Duration,
};

use axum::response::sse::{Event, Sse};
use futures_util::{stream::Stream, StreamExt};
use libp2p::{
    gossipsub::{self, Behaviour, IdentTopic, MessageAuthenticity},
    identity::Keypair,
    swarm::SwarmEvent,
    Multiaddr, Swarm, SwarmBuilder,
};

use crate::handlers::{
    create_log::write_log,
    structures::{GossipMessage, Transaction},
};

use super::server::Reciept;

async fn swarm() -> Swarm<gossipsub::Behaviour> {
    let keys = Keypair::generate_ecdsa();
    let topic = IdentTopic::new("sse");
    let privacy = MessageAuthenticity::Signed(keys.clone());
    let gossip_cfg_builder = libp2p::gossipsub::ConfigBuilder::default();

    let config = libp2p::gossipsub::ConfigBuilder::build(&gossip_cfg_builder).unwrap();
    config.duplicate_cache_time();
    let mut gossipsub: Behaviour = gossipsub::Behaviour::new(privacy, config).unwrap();
    gossipsub.subscribe(&topic).unwrap();

    let swarm_config = libp2p::swarm::Config::with_tokio_executor()
        .with_idle_connection_timeout(Duration::from_secs(u64::MAX));

    let mut swarm = SwarmBuilder::with_existing_identity(keys)
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
        .with_behaviour(|_key| gossipsub)
        .unwrap()
        .with_swarm_config(|_conf| swarm_config)
        .build();

    swarm
        .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .unwrap();

    let mut dial_addr = String::new();

    {
        let address_file = File::open("/etc/myaddress.dat").unwrap();
        let reader = BufReader::new(address_file);
        for i in reader.lines() {
            match i {
                Ok(addr) => {
                    if addr.trim().len() > 0 {
                        dial_addr.push_str(&addr);
                        break;
                    }
                }
                Err(e) => write_log(&format!("{}", e)),
            }
        }
    }

    let dial_multiaddr: Multiaddr = dial_addr.parse().unwrap();
    swarm.dial(dial_multiaddr).unwrap();

    swarm
}

pub async fn trx_sse() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
    let mut swarm = swarm().await;
    tokio::spawn(async move {
        loop {
            match swarm.select_next_some().await {
                SwarmEvent::Behaviour(gossipmsg) => match gossipmsg {
                    gossipsub::Event::Message { message, .. } => {
                        let msg = String::from_utf8(message.data).unwrap();
                        if let Ok(transaction) = serde_json::from_str::<Transaction>(&msg) {
                            let reciept = Reciept {
                                block_number: None,
                                hash: transaction.tx_hash,
                                from: transaction.output.output_data.sigenr_public_keys[0]
                                    .to_string()
                                    .clone(),
                                to: transaction.output.output_data.utxos[0]
                                    .output_unspent
                                    .public_key
                                    .clone(),
                                value: transaction.value,
                                fee: transaction.fee,
                                status: "Pending".to_string(),
                                description: "".to_string(),
                                date: transaction.date,
                            };
                            match tx.send(Ok(
                                Event::default().data(serde_json::to_string(&reciept).unwrap())
                            )) {
                                Ok(_) => {}
                                Err(_) => {
                                    write_log("error from send tx channel in transaction section!")
                                }
                            }
                        } else if let Ok(reciept) = serde_json::from_str::<Reciept>(&msg) {
                            match tx.send(Ok(
                                Event::default().data(serde_json::to_string(&reciept).unwrap())
                            )) {
                                Ok(_) => {}
                                Err(_) => {}
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    });

    Sse::new(stream)
}

pub async fn block_sse() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
    let mut swarm = swarm().await;
    tokio::spawn(async move {
        loop {
            match swarm.select_next_some().await {
                SwarmEvent::Behaviour(gossipmsg) => match gossipmsg {
                    gossipsub::Event::Message { message, .. } => {
                        let msg = String::from_utf8(message.data).unwrap();
                        if let Ok(gossipmessage) = serde_json::from_str::<GossipMessage>(&msg) {
                            match tx.send(Ok(Event::default()
                                .data(serde_json::to_string(&gossipmessage.block).unwrap())))
                            {
                                Ok(_) => {}
                                Err(_) => {}
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    });

    Sse::new(stream)
}
