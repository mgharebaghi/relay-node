mod handlers;
pub mod rpc;
use libp2p::Multiaddr;
pub use rpc::handle_requests;
use std::env::consts::OS;
use std::fs::File;
use std::io::{stdout, BufRead, BufReader};
use std::time::Duration;

use crossterm::execute;
use crossterm::style::{Print, ResetColor, SetForegroundColor, Stylize};
use handlers::create_log::write_log;
use handlers::handle_streams;
use handlers::structures::CustomBehav;
use handlers::structures::FullNodes;
use handlers::structures::Req;
use handlers::structures::Res;

use libp2p::{
    gossipsub::IdentTopic,
    identity::Keypair,
    request_response::{cbor, ProtocolSupport},
    PeerId, StreamProtocol, SwarmBuilder,
};

pub async fn run() {
    let mut wallet = String::new();
    let mut wallet_path = "";
    if OS == "linux" {
        wallet_path = "/etc/wallet.dat"
    } else if OS == "windows" {
        wallet_path = "wallet.dat"
    };
    let wallet_file = File::open(wallet_path);
    match wallet_file {
        Ok(file) => {
            let reader = BufReader::new(file);
            for addr in reader.lines() {
                let wallet_addr = addr.unwrap();
                if wallet_addr.trim().len() > 0 {
                    wallet.push_str(&wallet_addr);
                }
            }
        }
        Err(_) => {
            execute!(
                stdout(),
                SetForegroundColor(crossterm::style::Color::Red),
                Print("Could not find the wallet address file!\n".bold()),
                ResetColor
            )
            .ok();
            write_log("Could not find the wallet address file!".to_string());
            std::process::exit(404);
        }
    }

    let relay_topic = IdentTopic::new("relay");
    let clients_topic = IdentTopic::new("client");

    //generate peer keys and peer id for network
    let keypair = Keypair::generate_ecdsa();
    let local_peer_id = PeerId::from(keypair.public());

    //gossip protocol config
    let privacy = libp2p::gossipsub::MessageAuthenticity::Signed(keypair.clone());
    let gossip_cfg_builder = libp2p::gossipsub::ConfigBuilder::default();
    let gossip_cfg = libp2p::gossipsub::ConfigBuilder::build(&gossip_cfg_builder).unwrap();
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
    behaviour
        .gossipsub
        .subscribe(&clients_topic.clone())
        .unwrap();

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

    let mut connections: Vec<PeerId> = Vec::new();
    let mut relay_topic_subscribers: Vec<PeerId> = Vec::new();
    let mut client_topic_subscribers: Vec<PeerId> = Vec::new();
    let mut clients: Vec<PeerId> = Vec::new();
    let mut relays: Vec<PeerId> = Vec::new();
    let mut leader = String::new();
    let mut fullnode_subs: Vec<FullNodes> = Vec::new();
    let mut my_addresses = Vec::new();
    let mut sync = false;
    let mut syncing_blocks = Vec::new();

    handle_streams(
        local_peer_id,
        &mut swarm,
        clients_topic,
        &mut my_addresses,
        &mut relays,
        &mut clients,
        relay_topic,
        &mut connections,
        &mut relay_topic_subscribers,
        &mut client_topic_subscribers,
        &mut wallet,
        &mut leader,
        &mut fullnode_subs,
        &mut sync,
        &mut syncing_blocks,
    )
    .await;
}
