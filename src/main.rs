use std::{
    fs,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    net::Ipv4Addr, time::Duration
};

use libp2p::{
    futures::StreamExt,
    gossipsub::{IdentTopic, Message, TopicHash},
    identity::Keypair,
    request_response::{cbor, Event, ProtocolSupport, ResponseChannel},
    swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent},
    tcp, Multiaddr, PeerId, StreamProtocol, Swarm, Transport,
};
use rand::{seq::SliceRandom, Rng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Req {
    req: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Res {
    res: String,
}

#[derive(NetworkBehaviour)]
struct CustomBehav {
    keep_alive: libp2p::swarm::keep_alive::Behaviour,
    gossipsub: libp2p::gossipsub::Behaviour,
    req_res: cbor::Behaviour<Req, Res>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReqForReq {
    peer: Vec<PeerId>,
    req: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResForReq {
    peer: Vec<PeerId>,
    res: Res,
}

#[derive(Debug)]
struct Channels {
    peer: PeerId,
    channel: ResponseChannel<Res>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NewSyncNode {
    peerid: PeerId,
    msg: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutNode {
    peer_id: PeerId,
}

#[tokio::main]
async fn main() {
    let relay_topic = IdentTopic::new("relay");
    let clients_topic = IdentTopic::new("client");
    let keypair = Keypair::generate_ecdsa();
    let local_peer_id = PeerId::from(keypair.public());
    println!("peer id: {}", local_peer_id.clone());

    let tcp_transport = tcp::tokio::Transport::default();
    let transport = tcp_transport
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(libp2p::noise::Config::new(&keypair).unwrap())
        .multiplex(libp2p::yamux::Config::default())
        .boxed();

    let keep_alive = libp2p::swarm::keep_alive::Behaviour::default();

    let privacy = libp2p::gossipsub::MessageAuthenticity::Signed(keypair);
    let mut gossip_cfg_builder = libp2p::gossipsub::ConfigBuilder::default();
    gossip_cfg_builder.idle_timeout(Duration::from_secs(60 * 1000000));
    
    let gossip_cfg = libp2p::gossipsub::ConfigBuilder::build(&gossip_cfg_builder).unwrap();

    let mut gossipsub = libp2p::gossipsub::Behaviour::new(privacy, gossip_cfg).unwrap();
    gossipsub.subscribe(&relay_topic.clone()).unwrap();
    gossipsub.subscribe(&clients_topic.clone()).unwrap();

    let req_res = cbor::Behaviour::<Req, Res>::new(
        [(StreamProtocol::new("/mg/1.0"), ProtocolSupport::Full)],
        libp2p::request_response::Config::default(),
    );

    let behaviour = CustomBehav {
        keep_alive,
        gossipsub,
        req_res,
    };

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

//handle streams that come to swarm events and relays.dat file to add or remove addresses
async fn handle_streams(
    local_peer_id: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    clients_topic: IdentTopic,
    my_addresses: &mut Vec<String>,
    channels: &mut Vec<Channels>,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
    connections: &mut Vec<PeerId>,
    relay_topic_subscribers: &mut Vec<PeerId>,
    client_topic_subscriber: &mut Vec<PeerId>,
) {
    loop {
        let relays_file_exist = fs::metadata("relays.dat").is_ok();
        if relays_file_exist {
            let file = File::open("relays.dat").unwrap();
            let reader = BufReader::new(&file);
            let mut dial_addresses = Vec::new();
            for i in reader.lines() {
                let addr = i.unwrap();
                if addr.trim().len() > 0 {
                    let addresses: Multiaddr = addr.parse().unwrap();
                    if !addresses.to_string().contains(&local_peer_id.to_string()) {
                        dial_addresses.push(addresses);
                    }
                }
            }
            if dial_addresses.len() > 0 {
                let rnd_dial_addr = dial_addresses
                    .choose(&mut rand::thread_rng())
                    .unwrap()
                    .clone();
                swarm.dial(rnd_dial_addr.clone()).unwrap();
                println!("dialing with: {}\n---------------", rnd_dial_addr.clone());
            }
        }

        handle_events(
            swarm,
            local_peer_id,
            my_addresses,
            clients,
            channels,
            relays,
            clients_topic.clone(),
            relay_topic.clone(),
            &mut connections.clone(),
            relay_topic_subscribers,
            client_topic_subscriber,
        )
        .await;
    }

    //handle events of swarm in a loop
    async fn handle_events(
        mut swarm: &mut Swarm<CustomBehav>,
        local_peer_id: PeerId,
        my_addresses: &mut Vec<String>,
        mut clients: &mut Vec<PeerId>,
        mut channels: &mut Vec<Channels>,
        mut relays: &mut Vec<PeerId>,
        clients_topic: IdentTopic,
        relay_topic: IdentTopic,
        connections: &mut Vec<PeerId>,
        relay_topic_subscribers: &mut Vec<PeerId>,
        client_topic_subscriber: &mut Vec<PeerId>,
    ) {
        loop {
            match swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    let str_addr = address.clone().to_string();
                    let ipv4 = str_addr.split("/").nth(2).unwrap();
                    let ip:Ipv4Addr = ipv4.parse().unwrap();
                    if !ip.is_private() && ipv4 != "127.0.0.1" {
                        handle_new_listener_event(address, local_peer_id, my_addresses);
                    }
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    println!(
                        "connection stablishe with: {}\n------------------------",
                        peer_id
                    );
                    connections.push(peer_id);
                }
                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                    println!(
                        "dialing with: {}\nhas error: {:?}\n------------------------",
                        peer_id.unwrap(),
                        error
                    );
                    removed_peer_dialing_error(peer_id.unwrap());
                    break;
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    println!("connection closed with:\n{}\n-------------------", peer_id);
                    if client_topic_subscriber.contains(&peer_id) {
                        let index = client_topic_subscriber
                            .iter()
                            .position(|x| *x == peer_id)
                            .unwrap();
                        client_topic_subscriber.remove(index);
                    }
                    if clients.contains(&peer_id) {
                        let i_client = clients.iter().position(|i| i == &peer_id).unwrap();
                        clients.remove(i_client);
                        handle_out_node(
                            peer_id,
                            swarm,
                            clients_topic.clone(),
                            client_topic_subscriber,
                            relays,
                            clients,
                            relay_topic.clone(),
                        );
                        swarm
                            .behaviour_mut()
                            .gossipsub
                            .remove_explicit_peer(&peer_id);
                    }

                    if relay_topic_subscribers.contains(&peer_id) {
                        let i_relay_subscriber = relay_topic_subscribers
                            .iter()
                            .position(|pid| *pid == peer_id)
                            .unwrap();
                        relay_topic_subscribers.remove(i_relay_subscriber);
                    }

                    if relays.contains(&peer_id) {
                        let i_relay = relays.iter().position(|pid| pid == &peer_id).unwrap();
                        relays.remove(i_relay);

                        swarm
                            .behaviour_mut()
                            .gossipsub
                            .remove_explicit_peer(&peer_id);
                        removed_peer_dialing_error(peer_id);
                        println!("relay removed: {}\n-------------------", peer_id);
                        println!("relays after remove: {:?}\n-------------------", relays);
                        break;
                    }
                }
                SwarmEvent::Behaviour(custom_behav) => match custom_behav {
                    CustomBehavEvent::KeepAlive(keep_alive) => {
                        println!("keep alive: {:?}", keep_alive);
                    }
                    CustomBehavEvent::Gossipsub(gossipevent) => match gossipevent {
                        libp2p::gossipsub::Event::Message {
                            propagation_source,
                            message,
                            ..
                        } => {
                            handle_gossip_message(
                                propagation_source,
                                message,
                                local_peer_id,
                                clients,
                                relays,
                                swarm,
                                relay_topic.clone(),
                                connections,
                                relay_topic_subscribers,
                                my_addresses
                            );
                        }
                        libp2p::gossipsub::Event::Subscribed { peer_id, topic } => {
                            send_my_address_after_get_new_subscriber(
                                topic,
                                peer_id,
                                &mut swarm,
                                my_addresses.clone(),
                                relay_topic_subscribers,
                                connections,
                                clients,
                                client_topic_subscriber,
                            )
                        }
                        _ => (),
                    },
                    CustomBehavEvent::ReqRes(req_res) => {
                        println!("{:?}\n--------------------", req_res);
                        match req_res {
                            Event::Message { peer, message } => {
                                println!(
                                    "you get message from: {}\nmessage: {:?}\n-----------------",
                                    peer, message
                                );
                                match message {
                                    libp2p::request_response::Message::Request {
                                        channel,
                                        request,
                                        ..
                                    } => {
                                        handle_requests(
                                            request,
                                            &mut clients,
                                            &mut swarm,
                                            &mut channels,
                                            channel,
                                            &mut relays,
                                            peer,
                                            local_peer_id,
                                        );
                                    }
                                    libp2p::request_response::Message::Response {
                                        response,
                                        ..
                                    } => {
                                        handle_responses(response, local_peer_id, channels, swarm);
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                },
                _ => (),
            }
        }
    }
}

fn handle_new_listener_event(
    address: Multiaddr,
    local_peer_id: PeerId,
    my_addresses: &mut Vec<String>,
) {
    println!("listener: {}\n----------------", address);
    let my_full_addr = format!("{}/p2p/{}", address, local_peer_id);
    let exists = fs::metadata("relays.dat").is_ok();
    if exists {
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open("relays.dat")
            .unwrap();
        let mut buf_writer = BufWriter::new(&file);
        writeln!(buf_writer, "{}", my_full_addr.clone()).unwrap();
    } else {
        File::create("relays.dat").unwrap();
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open("relays.dat")
            .unwrap();
        let mut buf_writer = BufWriter::new(&file);
        let my_full_addr = format!("{}/p2p/{}", address, local_peer_id);
        writeln!(buf_writer, "{}", my_full_addr).unwrap();
    }
    my_addresses.push(my_full_addr);
}

fn handle_out_node(
    peerid: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    clients_topic: IdentTopic,
    client_topic_subscriber: &mut Vec<PeerId>,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
) {
    let outnode = OutNode { peer_id: peerid };
    let serialize_out_node = serde_json::to_string(&outnode).unwrap();
    if client_topic_subscriber.len() > 0 {
        swarm
            .behaviour_mut()
            .gossipsub
            .publish(clients_topic, serialize_out_node.as_bytes())
            .unwrap();
    }

    if relays.len() > 0 && clients.len() == 0 {
        swarm
            .behaviour_mut()
            .gossipsub
            .publish(relay_topic, "i dont have any clients".as_bytes())
            .unwrap();
    }
}

//handle gossip messages that recieved fom clients or relays
fn handle_gossip_message(
    propagation_source: PeerId,
    message: Message,
    local_peer_id: PeerId,
    clients: &mut Vec<PeerId>,
    relays: &mut Vec<PeerId>,
    swarm: &mut Swarm<CustomBehav>,
    relay_topic: IdentTopic,
    connections: &mut Vec<PeerId>,
    relay_topic_subscribers: &mut Vec<PeerId>,
    my_addresses: &mut Vec<String>
) {
    println!(
        "gossip messag from:\n{}\nmessag: {:?}\n--------------------",
        propagation_source,
        String::from_utf8(message.data.clone()).unwrap()
    );

    let msg = String::from_utf8(message.data.clone()).unwrap();

    if let Ok(addresses) = serde_json::from_str::<Vec<String>>(&msg) {
        let r_relay_file = File::open("relays.dat").unwrap();
        let reader = BufReader::new(r_relay_file);
        let mut old_addresses = Vec::new();
        for i in reader.lines() {
            let line = i.unwrap();
            old_addresses.push(line);
        }
        let relays_file = OpenOptions::new()
            .write(true)
            .append(true)
            .open("relays.dat")
            .unwrap();
        let mut buf_writer = BufWriter::new(relays_file);
        for i in addresses {
            if !i.contains(&local_peer_id.to_string()) && !old_addresses.contains(&i) {
                writeln!(buf_writer, "{}", i).unwrap();
            }

            if !my_addresses.contains(&i) {
                my_addresses.push(i);
            }
        }
    }

    //Relay announcement
    if let Ok(new_sync_node) = serde_json::from_str::<NewSyncNode>(&msg) {
        if !relay_topic_subscribers.contains(&propagation_source) {
            let new_sync_node_pid = new_sync_node.peerid;
            clients.push(new_sync_node_pid);
            if relay_topic_subscribers.len() > 0 && clients.len() == 1 {
                swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(relay_topic, "i have a client".as_bytes())
                    .unwrap();
                println!("im relay sent\n------------------------");
            }
            println!("clients after add new node:\n{:?}", clients);
        }
    }

    if msg == "i have a client".to_string() && connections.contains(&propagation_source) {
        if !relays.contains(&propagation_source) {
            relays.push(propagation_source);
            println!(
                "new relay: {}\nrelays: {:?}\n-------------------",
                propagation_source, relays
            );
        }
    }

    if msg == "i dont have any clients".to_string() && relays.contains(&propagation_source) {
        let index = relays
            .iter()
            .position(|relay| *relay == propagation_source)
            .unwrap();
        relays.remove(index);
    }
}

//handle requests that recieved from clients or relays
fn handle_requests(
    request: Req,
    clients: &mut Vec<PeerId>,
    swarm: &mut Swarm<CustomBehav>,
    channels: &mut Vec<Channels>,
    channel: ResponseChannel<Res>,
    relays: &mut Vec<PeerId>,
    peer: PeerId,
    local_peer_id: PeerId,
) {
    if clients.len() > 0 {
        let chnl = Channels { peer, channel };
        channels.push(chnl);
        println!("in clients bigger\n{:?}\n----------------", channels);
        println!("clients: {:?}\n----------------", clients);
        let random_client = clients.choose(&mut rand::thread_rng()).unwrap();
        swarm
            .behaviour_mut()
            .req_res
            .send_request(random_client, request);
    } else if relays.len() > 0 {
        let mut relays_without_req_sender: Vec<PeerId> = Vec::new();
        for i in 0..relays.len() {
            if relays[i] != peer {
                relays_without_req_sender.push(relays[i].clone());
                println!(
                    "relays without req push: {}\n------------",
                    relays[i].clone()
                );
            }
        }
        if relays_without_req_sender.len() > 0 {
            println!(
                "in relays bigger\n{:?}\n------------------",
                relays_without_req_sender
            );
            let chnl = Channels { peer, channel };
            channels.push(chnl);
            println!("{:?}\n----------------", channels);
            let mut original_req: ReqForReq = serde_json::from_str(&request.req).unwrap();
            original_req.peer.push(local_peer_id);
            let req = serde_json::to_string(&original_req).unwrap();
            let req_for_relay = Req { req };
            let random_relay = relays_without_req_sender
                .choose(&mut rand::thread_rng())
                .unwrap();
            // channels.push(channel);
            swarm
                .behaviour_mut()
                .req_res
                .send_request(random_relay, req_for_relay);
        } else {
            let response = Res {
                res: "You Are First Client".to_string(),
            };
            let original_req: ReqForReq = serde_json::from_str(&request.req).unwrap();
            let res = ResForReq {
                peer: original_req.peer,
                res: response,
            };
            let str_res = serde_json::to_string(&res).unwrap();
            let final_res = Res { res: str_res };
            swarm
                .behaviour_mut()
                .req_res
                .send_response(channel, final_res)
                .unwrap();
        }
    } else {
        let response = Res {
            res: "You Are First Client".to_string(),
        };
        let original_req: ReqForReq = serde_json::from_str(&request.req).unwrap();
        let res = ResForReq {
            peer: original_req.peer,
            res: response,
        };
        let str_res = serde_json::to_string(&res).unwrap();
        let final_res = Res { res: str_res };
        swarm
            .behaviour_mut()
            .req_res
            .send_response(channel, final_res)
            .unwrap();
    }
}

//send listener addresses to another relays and clients
fn send_my_address_after_get_new_subscriber(
    topic: TopicHash,
    peer_id: PeerId,
    swarm: &mut Swarm<CustomBehav>,
    my_addresses: Vec<String>,
    relay_topic_subscribers: &mut Vec<PeerId>,
    connections: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    client_topic_subscriber: &mut Vec<PeerId>,
) {
    println!(
        "this peer: {}\nsubscribe to: {}\n-------------------",
        peer_id, topic
    );
    if topic.to_string() == "relay".to_string() {
        relay_topic_subscribers.push(peer_id);
        if connections.contains(&peer_id) && clients.len() > 0 {
            swarm
                .behaviour_mut()
                .gossipsub
                .publish(topic.clone(), "i have a client".as_bytes())
                .unwrap();
        }
        let my_addresses_str = serde_json::to_string(&my_addresses).unwrap();
        swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), my_addresses_str.as_bytes())
            .unwrap();
    }

    if topic.to_string() == "client".to_string() && connections.contains(&peer_id) {
        client_topic_subscriber.push(peer_id);
        let my_addresses_str = serde_json::to_string(&my_addresses).unwrap();
        swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), my_addresses_str.as_bytes())
            .unwrap();
    }
}

//remove peer from relays.dat file when it disconnected
fn removed_peer_dialing_error(peerid: PeerId) {
    let file = File::open("relays.dat").unwrap();
    let reader = BufReader::new(&file);
    let mut lines = Vec::new();
    for i in reader.lines() {
        let line = i.unwrap();
        if !line.contains(&peerid.to_string()) {
            lines.push(line);
        }
    }
    let mut writer = File::create("relays.dat").unwrap();
    for line in lines {
        writeln!(writer, "{}", line).unwrap();
    }
}

fn handle_responses(
    response: Res,
    local_peer_id: PeerId,
    channels: &mut Vec<Channels>,
    swarm: &mut Swarm<CustomBehav>,
) {
    let mut res: ResForReq = serde_json::from_str(&response.res).unwrap();

    if res.peer.last().unwrap() == &local_peer_id {
        res.peer.pop();
        let new_res = serde_json::to_string(&res).unwrap();
        let new_response = Res { res: new_res };
        let index = channels
            .iter()
            .position(|channel| channel.peer == res.peer.last().unwrap().clone())
            .unwrap();
        swarm
            .behaviour_mut()
            .req_res
            .send_response(channels.remove(index).channel, new_response)
            .unwrap();
    } else {
        let index = channels
            .iter()
            .position(|channel| channel.peer == res.peer.last().unwrap().clone())
            .unwrap();
        swarm
            .behaviour_mut()
            .req_res
            .send_response(channels.remove(index).channel, response)
            .unwrap();
    }
}
