use std::{fs::File, io::{BufRead, BufReader}, time::Duration};

use libp2p::{
    identity::Keypair,
    swarm::NetworkBehaviour,
    Multiaddr, Swarm, SwarmBuilder,
};

use crate::handlers::create_log::write_log;

pub trait MiddleSwarmConf {
    async fn new() -> Swarm<MyBehaviour>;
}

#[derive(NetworkBehaviour)]
pub struct MyBehaviour {
    pub gossipsub: libp2p::gossipsub::Behaviour,
}

impl MiddleSwarmConf for MyBehaviour {
    async fn new() -> Swarm<Self> {
        //generate peer keys and peer id for network
        let keypair = Keypair::generate_ecdsa();

        //gossip protocol config
        let privacy = libp2p::gossipsub::MessageAuthenticity::Signed(keypair.clone());
        let gossip_cfg = libp2p::gossipsub::ConfigBuilder::default().build().unwrap();
        gossip_cfg.duplicate_cache_time();
        let gossipsub = libp2p::gossipsub::Behaviour::new(privacy, gossip_cfg).unwrap();

        //Definition of behavior
        let behaviour = MyBehaviour { gossipsub };

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

        let my_addr_file = File::open("/etc/myaddress.dat");
        if let Ok(file) = my_addr_file {
            let reader = BufReader::new(file);
            for line in reader.lines() {
                if let Ok(addr) = line {
                    let multiaddr:Multiaddr = addr.parse().unwrap();
                    match swarm.dial(multiaddr) {
                        Ok(_) => {
                            write_log(&format!("dilaing with: {}", addr));
                        }
                        Err(_) => {

                        }
                    }
                }
            }
        }
        
        swarm
    }
}
