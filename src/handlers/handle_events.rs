use std::io::stdout;
use std::net::Ipv4Addr;

use libp2p::futures::StreamExt;
use libp2p::{gossipsub::IdentTopic, request_response::Event, swarm::SwarmEvent, PeerId, Swarm};

use super::gossip_messages::handle_gossip_message;
use super::handle_listeners::handle;
use super::outnodes::handle_outnode;
use super::remove_relays::remove_peer;
use super::requests::handle_requests;
use super::responses::handle_responses;
use super::send_address::send_address;
use super::structures::{Channels, CustomBehav, CustomBehavEvent};
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
};

pub async fn events(
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
    wallet: &mut String,
) {
    loop {
        match swarm.next().await.unwrap() {
            SwarmEvent::NewListenAddr { address, .. } => {
                let str_addr = address.clone().to_string();
                let ipv4 = str_addr.split("/").nth(2).unwrap();
                let ip: Ipv4Addr = ipv4.parse().unwrap();
                if !ip.is_private() && ipv4 != "127.0.0.1" {
                    handle(address, local_peer_id, my_addresses);
                } else {
                    execute!(
                        stdout(),
                        SetForegroundColor(Color::Magenta),
                        Print("Find a local IP!\n"),
                        ResetColor
                    )
                    .unwrap();
                }
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                connections.push(peer_id);
                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, .. } => {
                execute!(
                    stdout(),
                    SetForegroundColor(Color::Red),
                    Print("Dialing failed with:\n".bold()),
                    ResetColor
                ).unwrap();
                println!("{}", peer_id.unwrap());
                remove_peer(peer_id.unwrap());
                for i in relays.clone() {
                    if peer_id.unwrap() == i {
                        let i_relay = relays
                            .iter()
                            .position(|pid| pid == &peer_id.unwrap())
                            .unwrap();
                        relays.remove(i_relay);
                    }
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
                    let i_relay = relays.iter().position(|pid| pid == &peer_id).unwrap();
                    relays.remove(i_relay);

                    swarm
                        .behaviour_mut()
                        .gossipsub
                        .remove_explicit_peer(&peer_id);
                    remove_peer(peer_id);
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
                        );
                    }
                    None => {}
                }

                if relay_topic_subscribers.contains(&peer_id) {
                    let i_relay_subscriber = relay_topic_subscribers
                        .iter()
                        .position(|pid| *pid == peer_id)
                        .unwrap();
                    relay_topic_subscribers.remove(i_relay_subscriber);
                }
            }
            SwarmEvent::Behaviour(custom_behav) => match custom_behav {
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
                            my_addresses,
                        );
                    }
                    libp2p::gossipsub::Event::Subscribed { peer_id, topic } => send_address(
                        topic,
                        peer_id,
                        &mut swarm,
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
                                &mut clients,
                                &mut swarm,
                                &mut channels,
                                channel,
                                &mut relays,
                                peer,
                                local_peer_id,
                                wallet,
                            );
                        }
                        libp2p::request_response::Message::Response { response, .. } => {
                            handle_responses(
                                response,
                                local_peer_id,
                                channels,
                                swarm,
                                client_topic_subscriber,
                                relay_topic_subscribers,
                            );
                        }
                    },
                    _ => (),
                },
            },
            _ => (),
        }
    }
}
