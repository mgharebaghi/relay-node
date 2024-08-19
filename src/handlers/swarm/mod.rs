use std::{pin::Pin, time::Duration};

use addresses::Addresses;
use libp2p::{
    gossipsub::{Config, IdentTopic, MessageAuthenticity},
    identity::Keypair,
    request_response::{cbor, ProtocolSupport},
    swarm::NetworkBehaviour,
    PeerId, StreamProtocol, Swarm, SwarmBuilder,
};
use mongodb::Database;
use serde::{Deserialize, Serialize};

use super::tools::relay::RelayNumber;
mod addresses;

#[derive(Debug, Serialize, Deserialize)]
pub struct Req {
    pub req: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Res {
    pub res: String,
}

#[derive(NetworkBehaviour)]
pub struct CentichainBehaviour {
    pub gossipsub: libp2p::gossipsub::Behaviour,
    pub reqres: cbor::Behaviour<Req, Res>,
}

impl CentichainBehaviour {
    pub async fn new() -> (Pin<Box<Swarm<CentichainBehaviour>>>, PeerId) {
        //generate keypair for get peerid to comunicate in the network
        let keypair = Keypair::generate_ed25519();
        let peerid = PeerId::from_public_key(&keypair.public());

        //topic for subscribing to a gossipsub protocol group
        let topic = IdentTopic::new("client");

        //configure gossipsub protocol
        let auth = MessageAuthenticity::Signed(keypair.clone());
        let conf = Config::default();
        let mut gossipsub = libp2p::gossipsub::Behaviour::new(auth, conf).unwrap();
        gossipsub.subscribe(&topic).unwrap();

        //configure request response protocol
        let reqres = cbor::Behaviour::<Req, Res>::new(
            [(StreamProtocol::new("/mg/1.0"), ProtocolSupport::Full)],
            libp2p::request_response::Config::default(),
        );

        //swarm behaviour
        let behaviour = CentichainBehaviour { gossipsub, reqres };

        //it will configure swarm to has stable connection without time limitation
        let swarmconf = libp2p::swarm::Config::with_tokio_executor()
            .with_idle_connection_timeout(Duration::from_secs(u64::MAX));
        //build new swarm with tcp, websocket, quic protocols with centichain behaviour(gossipsub & reqres) and tokio async runtime
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
            .with_swarm_config(|_config| swarmconf)
            .build();

        //listen on a random port on all IPs in a system
        swarm
            .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
            .unwrap();

        //return a tuple that has a pined swarm on heap and a peerid
        (Box::pin(swarm), peerid)
    }

    //dialing to realys as random
    pub async fn dial<'a>(
        swarm: &mut Swarm<CentichainBehaviour>,
        db: &Database,
    ) -> Result<RelayNumber, &'a str> {
        match Addresses::contact(swarm, db).await {
            Ok(relay_number) => Ok(relay_number),
            Err(e) => Err(e),
        }
    }
}
