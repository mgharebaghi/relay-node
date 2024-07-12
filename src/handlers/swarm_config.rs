use std::{pin::Pin, time::Duration};

use libp2p::{
    gossipsub::IdentTopic,
    identity::Keypair,
    request_response::{cbor, ProtocolSupport},
    swarm::NetworkBehaviour,
    Multiaddr, PeerId, StreamProtocol, Swarm, SwarmBuilder,
};

use super::structures::{Req, Res};

pub trait SwarmConf {
    async fn new() -> (Pin<Box<Swarm<CustomBehav>>>, PeerId);
}

#[derive(NetworkBehaviour)]
pub struct CustomBehav {
    pub gossipsub: libp2p::gossipsub::Behaviour,
    pub req_res: cbor::Behaviour<Req, Res>,
}

impl SwarmConf for CustomBehav {
    async fn new() -> (Pin<Box<Swarm<Self>>>, PeerId) {
        let relay_topic = IdentTopic::new("relay");
        let clients_topic = IdentTopic::new("client");

        //generate peer keys and peer id for network
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());

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
        let mut behaviour = CustomBehav { gossipsub, req_res };

        behaviour.gossipsub.subscribe(&relay_topic.clone()).unwrap();
        behaviour.gossipsub.subscribe(&clients_topic.clone()).unwrap();

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
            .with_swarm_config(|_conf| swarm_config)
            .build();

        let listener: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
        swarm.listen_on(listener).unwrap();
        (Box::pin(swarm), local_peer_id)
    }
}
