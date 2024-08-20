use std::{
    fs::File,
    io::{BufRead, BufReader},
    pin::Pin,
    time::Duration,
};

use libp2p::{
    identity::Keypair,
    request_response::{cbor, ProtocolSupport},
    swarm::NetworkBehaviour,
    Multiaddr, StreamProtocol, Swarm, SwarmBuilder,
};

use crate::handlers::swarm::{Req, Res};

pub trait MiddleSwarmConf {
    async fn new() -> Pin<Box<Swarm<MyBehaviour>>>;
}

#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub gossipsub: libp2p::gossipsub::Behaviour,
    pub req_res: cbor::Behaviour<Req, Res>,
}

impl MiddleSwarmConf for MyBehaviour {
    async fn new() -> Pin<Box<Swarm<Self>>> {
        //generate peer keys and peer id for network
        let keypair = Keypair::generate_ecdsa();

        //gossip protocol config
        let privacy = libp2p::gossipsub::MessageAuthenticity::Signed(keypair.clone());
        let gossip_cfg = libp2p::gossipsub::ConfigBuilder::default().build().unwrap();
        gossip_cfg.duplicate_cache_time();
        let gossipsub = libp2p::gossipsub::Behaviour::new(privacy, gossip_cfg).unwrap();

        //request and response protocol config
        let req_res = cbor::Behaviour::<Req, Res>::new(
            [(StreamProtocol::new("/mg/1.0"), ProtocolSupport::Full)],
            libp2p::request_response::Config::default(),
        );

        //Definition of behavior
        let behaviour = MyBehaviour { gossipsub, req_res };

        //config swarm
        let swarm_config = libp2p::swarm::Config::with_tokio_executor()
            .with_idle_connection_timeout(Duration::from_secs(u64::MAX));

            let mut swarm = SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                Default::default(),
                (libp2p::tls::Config::new, libp2p::noise::Config::new),
                libp2p::yamux::Config::default,
            )
            .unwrap()
            .with_websocket(
                (libp2p::tls::Config::new, libp2p::noise::Config::new),
                libp2p::yamux::Config::default,
            )
            .await
            .unwrap()
            .with_behaviour(|_key| behaviour)
            .unwrap()
            .with_swarm_config(|_config| swarm_config)
            .build();

        let listener: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
        swarm.listen_on(listener).unwrap();

        let my_addr_file = File::open("/etc/myaddress.dat");
        if let Ok(file) = my_addr_file {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(addr) = line {
                    let multiaddr: Multiaddr = addr.parse().unwrap();
                    match swarm.dial(multiaddr) {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
            }
        }

        Box::pin(swarm)
    }
}
