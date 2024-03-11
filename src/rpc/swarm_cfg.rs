use std::time::Duration;

use libp2p::{
    identity::Keypair,
    request_response::{cbor, ProtocolSupport},
    swarm::NetworkBehaviour,
    Multiaddr, StreamProtocol, Swarm, SwarmBuilder,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Req {
    pub req: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Res {
    pub res: String,
}

pub trait SwarmConf {
    async fn new() -> Swarm<CostumBehav>;
}

#[derive(NetworkBehaviour)]
pub struct CostumBehav {
    pub req_res: cbor::Behaviour<Req, Res>,
}

impl SwarmConf for CostumBehav {
    async fn new() -> Swarm<Self> {
        //generate peer keys and peer id for network
        let keypair = Keypair::generate_ecdsa();

        //request and response protocol config
        let req_res = cbor::Behaviour::<Req, Res>::new(
            [(StreamProtocol::new("/mg/1.0"), ProtocolSupport::Full)],
            libp2p::request_response::Config::default(),
        );

        //Definition of behavior
        let behaviour = CostumBehav { req_res };

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
        swarm
    }
}
