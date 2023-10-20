mod handlers;
use handlers::handle_streams;
use handlers::structures::Channels;
use handlers::structures::CustomBehav;
use handlers::structures::Req;
use handlers::structures::Res;

use env_logger::Env;
use libp2p::{
    gossipsub::IdentTopic,
    identity::Keypair,
    request_response::{cbor, ProtocolSupport},
    swarm::SwarmBuilder,
    tcp, Multiaddr, PeerId, StreamProtocol, Transport,
};

pub async fn run() {
    //initialize logger for terminal
    env_logger::Builder::from_env(
        Env::default()
            .default_filter_or("warn")
            .default_filter_or("error")
            .default_filter_or("info"),
    )
    .init();

    let relay_topic = IdentTopic::new("relay");
    let clients_topic = IdentTopic::new("client");

    //generate peer keys and peer id for network
    let keypair = Keypair::generate_ecdsa();
    let local_peer_id = PeerId::from(keypair.public());

    //config transport as TCP
    let tcp_transport = tcp::tokio::Transport::default();
    let transport = tcp_transport
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(libp2p::noise::Config::new(&keypair).unwrap())
        .multiplex(libp2p::yamux::Config::default())
        .boxed();

    //initial keep alive behaviour for stable connection
    let keep_alive = libp2p::swarm::keep_alive::Behaviour::default();

    //gossip protocol config
    let privacy = libp2p::gossipsub::MessageAuthenticity::Signed(keypair);
    let gossip_cfg_builder = libp2p::gossipsub::ConfigBuilder::default();
    let gossip_cfg = libp2p::gossipsub::ConfigBuilder::build(&gossip_cfg_builder).unwrap();
    let gossipsub = libp2p::gossipsub::Behaviour::new(privacy, gossip_cfg).unwrap();

    //request and response protocol config
    let req_res = cbor::Behaviour::<Req, Res>::new(
        [(StreamProtocol::new("/mg/1.0"), ProtocolSupport::Full)],
        libp2p::request_response::Config::default(),
    );

    //Definition of behavior
    let mut behaviour = CustomBehav {
        keep_alive,
        gossipsub,
        req_res,
    };

    behaviour.gossipsub.subscribe(&relay_topic.clone()).unwrap();
    behaviour
        .gossipsub
        .subscribe(&clients_topic.clone())
        .unwrap();

    //config swarm
    let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build();
    let listener: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
    swarm.listen_on(listener).unwrap();

    let mut connections: Vec<PeerId> = Vec::new();
    let mut relay_topic_subscribers: Vec<PeerId> = Vec::new();
    let mut client_topic_subscribers: Vec<PeerId> = Vec::new();
    let mut clients: Vec<PeerId> = Vec::new();
    let mut relays: Vec<PeerId> = Vec::new();

    let mut channels: Vec<Channels> = Vec::new();
    let mut my_addresses = Vec::new();

    handle_streams(
        local_peer_id,
        &mut swarm,
        clients_topic,
        &mut my_addresses,
        &mut channels,
        &mut relays,
        &mut clients,
        relay_topic,
        &mut connections,
        &mut relay_topic_subscribers,
        &mut client_topic_subscribers,
    )
    .await;
}
