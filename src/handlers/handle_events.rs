use std::net::Ipv4Addr;
use std::process::Command;

use libp2p::core::transport::ListenerId;
use libp2p::futures::StreamExt;
use libp2p::{gossipsub::IdentTopic, request_response::Event, swarm::SwarmEvent, PeerId, Swarm};

use super::create_log::write_log;
use super::gossip_messages::handle_gossip_message;
use super::handle_listeners::{handle, send_addr_to_server};
use super::outnodes::handle_outnode;
use super::reciept::insert_reciept;
use super::recieved_block::verifying_block;
use super::remove_relays::remove_peer;
use super::requests::handle_requests;
use super::responses::handle_responses;
use super::send_address::send_address;
use super::structures::{
    Channels, CustomBehav, CustomBehavEvent, FullNodes, GetGossipMsg, GossipMessage, Req,
    Transaction,
};
use super::syncing::syncing;

#[derive(Debug)]
struct Listeners {
    id: Vec<ListenerId>,
}

pub async fn events(
    swarm: &mut Swarm<CustomBehav>,
    local_peer_id: PeerId,
    my_addresses: &mut Vec<String>,
    clients: &mut Vec<PeerId>,
    channels: &mut Vec<Channels>,
    relays: &mut Vec<PeerId>,
    clients_topic: IdentTopic,
    relay_topic: IdentTopic,
    connections: &mut Vec<PeerId>,
    relay_topic_subscribers: &mut Vec<PeerId>,
    client_topic_subscriber: &mut Vec<PeerId>,
    wallet: &mut String,
    leader: &mut String,
    fullnodes: &mut Vec<FullNodes>,
    sync: &mut bool,
    dialed_addr: String,
    syncing_blocks: &mut Vec<GetGossipMsg>,
) {
    let mut listeners = Listeners { id: Vec::new() };

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr {
                address,
                listener_id,
            } => {
                let str_addr = address.clone().to_string();
                let ipv4 = str_addr.split("/").nth(2).unwrap();
                let ip: Ipv4Addr = ipv4.parse().unwrap();
                if !ip.is_private() && ipv4 != "127.0.0.1" {
                    handle(address, local_peer_id, my_addresses).await;
                    if *sync {
                        println!("sync new listener");
                        send_addr_to_server(my_addresses[0].clone()).await;
                    }
                }

                listeners.id.push(listener_id);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                if !*sync && dialed_addr.contains(&peer_id.to_string()) {
                    match syncing(dialed_addr.clone()).await {
                        Ok(_) => {
                            let fullnodes_req = Req {
                                req: "fullnodes".to_string(),
                            };
                            match Command::new("mongodump")
                                .arg("--db")
                                .arg("Blockchain")
                                .arg("--out")
                                .arg("/etc/dump")
                                .output()
                            {
                                Ok(_) => {}
                                Err(e) => write_log(format!("{:?}", e)),
                            }
                            swarm
                                .behaviour_mut()
                                .req_res
                                .send_request(&peer_id, fullnodes_req);
                        }
                        Err(_) => break,
                    }
                }

                if *sync {
                    match Command::new("mongodump")
                        .arg("--db")
                        .arg("Blockchain")
                        .arg("--out")
                        .arg("/etc/dump")
                        .output()
                    {
                        Ok(_) => {}
                        Err(e) => write_log(format!("{:?}", e)),
                    }
                }
                connections.push(peer_id);
                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, .. } => {
                write_log(format!("Dialing failed with: {}", peer_id.unwrap()));
                remove_peer(peer_id.unwrap(), my_addresses).await;
                for i in relays.clone() {
                    if peer_id.unwrap() == i {
                        match relays.iter().position(|pid| pid == &peer_id.unwrap()) {
                            Some(index) => {
                                relays.remove(index);
                            }
                            None => {}
                        }
                    }
                }

                for listener in listeners.id {
                    swarm.remove_listener(listener);
                }
                break;
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                let index = client_topic_subscriber.iter().position(|c| *c == peer_id);
                match index {
                    Some(i) => {
                        client_topic_subscriber.remove(i);
                    }
                    None => {}
                }

                if relays.contains(&peer_id) {
                    match relays.iter().position(|pid| pid == &peer_id) {
                        Some(index) => {
                            relays.remove(index);
                        }
                        None => {}
                    }

                    swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                    remove_peer(peer_id, my_addresses).await;
                    for listener in listeners.id {
                        swarm.remove_listener(listener);
                    }
                    break;
                }

                let index = clients.iter().position(|c| *c == peer_id);
                match index {
                    Some(i) => {
                        clients.remove(i);
                        swarm
                            .behaviour_mut()
                            .gossipsub
                            .remove_explicit_peer(&peer_id);
                        handle_outnode(
                            peer_id,
                            swarm,
                            clients_topic.clone(),
                            client_topic_subscriber,
                            relays,
                            clients,
                            relay_topic.clone(),
                            my_addresses,
                            fullnodes,
                        )
                        .await;
                    }
                    None => {}
                }

                if relay_topic_subscribers.contains(&peer_id) {
                    let i_relay_subscriber = relay_topic_subscribers
                        .iter()
                        .position(|pid| *pid == peer_id);
                    match i_relay_subscriber {
                        Some(index) => {
                            relay_topic_subscribers.remove(index);
                        }
                        None => {}
                    }
                }
            }
            SwarmEvent::Behaviour(custom_behav) => match custom_behav {
                CustomBehavEvent::Gossipsub(gossipevent) => match gossipevent {
                    libp2p::gossipsub::Event::Message {
                        propagation_source,
                        message,
                        ..
                    } => {
                        if *sync {
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
                                my_addresses,
                                leader,
                                fullnodes,
                                clients_topic.clone(),
                                client_topic_subscriber,
                            )
                            .await;
                        } else {
                            let str_msg = String::from_utf8(message.data).unwrap();
                            if let Ok(gossipmsg) = serde_json::from_str::<GossipMessage>(&str_msg) {
                                let new_gossip = GetGossipMsg {
                                    gossip: gossipmsg,
                                    propagation_source: propagation_source,
                                };
                                syncing_blocks.push(new_gossip);
                            } else if let Ok(transaction) =
                                serde_json::from_str::<Transaction>(&str_msg)
                            {
                                insert_reciept(
                                    transaction,
                                    None,
                                    "pending".to_string(),
                                    "".to_string(),
                                )
                                .await;
                            }
                        }
                    }
                    libp2p::gossipsub::Event::Subscribed { peer_id, topic } => send_address(
                        topic,
                        peer_id,
                        swarm,
                        my_addresses.clone(),
                        relay_topic_subscribers,
                        connections,
                        clients,
                        client_topic_subscriber,
                    ),
                    _ => (),
                },
                CustomBehavEvent::ReqRes(req_res) => match req_res {
                    Event::Message { peer, message } => match message {
                        libp2p::request_response::Message::Request {
                            channel, request, ..
                        } => {
                            handle_requests(
                                request,
                                clients,
                                swarm,
                                channels,
                                channel,
                                relays,
                                peer,
                                local_peer_id,
                                wallet,
                                clients_topic.clone(),
                                fullnodes,
                            )
                            .await;
                        }
                        libp2p::request_response::Message::Response { response, .. } => {
                            if let Ok(fullnode_subs) =
                                serde_json::from_str::<Vec<FullNodes>>(&response.res)
                            {
                                for fullnode in fullnode_subs.clone() {
                                    fullnodes.push(fullnode)
                                }
                                if syncing_blocks.len() > 0 {
                                    for gossipmsg in syncing_blocks.clone() {
                                        let str_msg =
                                            &serde_json::to_string(&gossipmsg.gossip).unwrap();
                                        verifying_block(
                                            str_msg,
                                            leader,
                                            &mut fullnode_subs.clone(),
                                            swarm,
                                            gossipmsg.propagation_source,
                                            clients_topic.clone(),
                                            client_topic_subscriber,
                                            relays,
                                            clients,
                                            relay_topic.clone(),
                                            my_addresses,
                                        )
                                        .await;
                                    }
                                }

                                *sync = true;
                                send_addr_to_server(my_addresses[0].clone()).await;
                            } else {
                                handle_responses(
                                    response,
                                    local_peer_id,
                                    channels,
                                    swarm,
                                    client_topic_subscriber,
                                    relay_topic_subscribers,
                                )
                                .await;
                            }
                        }
                    },
                    _ => (),
                },
            },
            _ => (),
        }
    }
}
