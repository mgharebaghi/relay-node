use std::net::Ipv4Addr;
use std::process::Command;

use libp2p::core::transport::ListenerId;
use libp2p::futures::StreamExt;
// use futures::stream::TryStreamExt;
use libp2p::Multiaddr;
use libp2p::{gossipsub::IdentTopic, request_response::Event, swarm::SwarmEvent, PeerId, Swarm};
use mongodb::bson::{doc, Document};
use mongodb::{Collection, Database};

use super::create_log::write_log;
use super::get_addresses::get_addresses;
use super::gossip_messages::handle_gossip_message;
use super::handle_listeners::{handle, send_addr_to_server};
use super::outnodes::handle_outnode;
use super::reciept::insert_reciept;
use super::recieved_block::verifying_block;
use super::remove_relays::remove_peer;
use super::requests::handle_requests;
use super::send_address::send_address;
use super::structures::{GetGossipMsg, GossipMessage, Transaction};
use super::swarm_config::{CustomBehav, CustomBehavEvent};
use super::syncing::syncing;

#[derive(Debug)]
pub struct Listeners {
    pub id: Vec<ListenerId>,
}

#[derive(Debug)]
struct HaveClient {
    have: bool,
    pid: String
}

pub async fn events(
    mut swarm: &mut Swarm<CustomBehav>,
    local_peer_id: PeerId,
    my_addresses: &mut Vec<String>,
    clients: &mut Vec<PeerId>,
    relays: &mut Vec<PeerId>,
    clients_topic: IdentTopic,
    relay_topic: IdentTopic,
    connections: &mut Vec<PeerId>,
    relay_topic_subscribers: &mut Vec<PeerId>,
    client_topic_subscriber: &mut Vec<PeerId>,
    wallet: &mut String,
    leader: &mut String,
    sync: &mut bool,
    dialed_addr: &mut Vec<String>,
    syncing_blocks: &mut Vec<GetGossipMsg>,
    im_first: &mut bool,
    db: Database,
) {
    let mut listeners = Listeners { id: Vec::new() };
    let mut in_syncing = false;
    let mut have_client = HaveClient {
        have: false,
        pid: String::new()
    };
    //check swarm events that come from libp2p
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
                        send_addr_to_server(my_addresses[0].clone()).await;
                    }
                } else {
                    write_log("Find private IP!");
                }

                listeners.id.push(listener_id);
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                if *sync {
                    match Command::new("mongodump")
                        .arg("--db")
                        .arg("Blockchain")
                        .arg("--out")
                        .arg("/etc/dump")
                        .output()
                    {
                        Ok(_) => {
                            match Command::new("zip")
                                .arg("-r")
                                .arg("/home/blockchain.zip")
                                .arg("/etc/dump/Blockchain")
                                .output()
                            {
                                Ok(_) => {}
                                Err(e) => write_log(&format!("{:?}", e)),
                            }
                        }
                        Err(e) => write_log(&format!("{:?}", e)),
                    }
                }
                connections.push(peer_id);
                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, .. } => {
                write_log(&format!("dialing failed with: {}", peer_id.unwrap()));
                remove_peer(peer_id.unwrap()).await;
                let dialed_index = dialed_addr
                    .iter()
                    .position(|dialed| dialed.contains(&peer_id.unwrap().to_string()));
                match dialed_index {
                    Some(index) => {
                        dialed_addr.remove(index);
                    }
                    None => {}
                }
                if dialed_addr.len() < 1 {
                    leader.clear();
                    connections.clear();
                    client_topic_subscriber.clear();
                    relay_topic_subscribers.clear();
                    clients.clear();
                    relays.clear();
                    dialed_addr.clear();
                    syncing_blocks.clear();
                    my_addresses.clear();
                    *sync = false;
                    break;
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                handle_outnode(
                    peer_id,
                    &mut swarm,
                    clients_topic.clone(),
                    relays,
                    clients,
                    relay_topic.clone(),
                    leader,
                    relay_topic_subscribers,
                    client_topic_subscriber,
                    im_first,
                    dialed_addr,
                    db.clone(),
                )
                .await;

                //break for dial with other relays if there is not connection with any relays
                if !*im_first && relays.len() < 1 {
                    write_log("going for break in removed dialed addresses");
                    write_log(&format!(
                        "relay topic subscribers:\n{:?}",
                        relay_topic_subscribers
                    ));

                    for connected in connections.clone() {
                        swarm.disconnect_peer_id(connected.clone()).unwrap();
                    }
                    leader.clear();
                    connections.clear();
                    client_topic_subscriber.clear();
                    relay_topic_subscribers.clear();
                    clients.clear();
                    relays.clear();
                    dialed_addr.clear();
                    syncing_blocks.clear();
                    my_addresses.clear();
                    *sync = false;
                    break;
                }
            }
            SwarmEvent::Behaviour(custom_behav) => match custom_behav {
                CustomBehavEvent::Gossipsub(gossipevent) => match gossipevent {
                    libp2p::gossipsub::Event::Message {
                        propagation_source,
                        message,
                        ..
                    } => {
                        let str_msg = String::from_utf8(message.data.clone()).unwrap();
                        if *sync {
                            handle_gossip_message(
                                propagation_source,
                                local_peer_id,
                                message,
                                clients,
                                relays,
                                &mut swarm,
                                relay_topic.clone(),
                                connections,
                                relay_topic_subscribers,
                                my_addresses,
                                leader,
                                db.clone(),
                            )
                            .await;
                        } else {
                            write_log("gossip message recieved while not syncing");
                            write_log(&format!("gossip message: {:?}", str_msg));
                            //insert gossip message to syncing blocks because the relay is not sync
                            if let Ok(gossipmsg) = serde_json::from_str::<GossipMessage>(&str_msg) {
                                write_log("gossip message recieved while syncing is GossipMessage");
                                let new_gossip = GetGossipMsg {
                                    gossip: gossipmsg.clone(),
                                    propagation_source: propagation_source,
                                };
                                syncing_blocks.push(new_gossip);
                                write_log("syncing blocks pushed");
                                write_log(&format!(
                                    "syncing block hash: {:?}",
                                    gossipmsg.block.header.blockhash
                                ));
                            }
                            //insert transaction reciept to db before syncing
                            if let Ok(transaction) = serde_json::from_str::<Transaction>(&str_msg) {
                                write_log("gossip message recieved while syncing is Transaction");
                                insert_reciept(
                                    transaction,
                                    None,
                                    "pending".to_string(),
                                    "".to_string(),
                                    db.clone(),
                                )
                                .await;
                                write_log("reciept inserted while syncing");
                            }
                            //get new relays before syncing
                            if let Ok(addresses) = serde_json::from_str::<Vec<String>>(&str_msg) {
                                get_addresses(addresses, local_peer_id, my_addresses);
                            }

                            if str_msg == "i have a client".to_string() {
                                if connections.contains(&propagation_source)
                                    && !relays.contains(&propagation_source)
                                {
                                    relays.push(propagation_source);
                                    write_log("new relay add");

                                    if !have_client.have {
                                        have_client.have = true;
                                        have_client.pid = propagation_source.to_string();
                                    }
                                }
                            }

                            //start syncing after get first i have client message and client topic has node
                            if have_client.have && client_topic_subscriber.len() > 0 && !*sync && !in_syncing
                            {
                                in_syncing = true;
                                let mut addr = String::new();
                                for add in dialed_addr.clone() {
                                    if add.contains(&have_client.pid) {
                                        addr = add.clone();
                                        break;
                                    }
                                }

                                match syncing(addr.clone(), db.clone()).await {
                                    Ok(_) => {
                                        write_log("syncing completed");
                                        let mut set_sync = true;
                                        if syncing_blocks.len() > 0 {
                                            for gossipmsg in syncing_blocks.clone() {
                                                let str_msg =
                                                    &serde_json::to_string(&gossipmsg.gossip)
                                                        .unwrap();
                                                match verifying_block(str_msg, leader, db.clone())
                                                    .await
                                                {
                                                    Ok(_) => {
                                                        write_log("verifying block before syncing");
                                                    }
                                                    Err(e) => {
                                                        if e != "reject" {
                                                            set_sync = false;
                                                            write_log("verifying block error in syncing blocks of handle events(line 351)");
                                                            write_log(&format!(
                                                                "block insert error: {}",
                                                                e
                                                            ));
                                                            //remove node from fullnodes list because its block is wrong!
                                                            let validators_coll: Collection<
                                                                Document,
                                                            > = db.collection("validators");
                                                            let gossipmsg: GossipMessage =
                                                                serde_json::from_str(&str_msg)
                                                                    .unwrap();
                                                            let filter = doc! {"peer_id": gossipmsg.block.header.validator};
                                                            let cursor = validators_coll
                                                                .find_one(filter, None)
                                                                .await;
                                                            if let Ok(opt) = cursor {
                                                                if let Some(doc) = opt {
                                                                    validators_coll
                                                                        .delete_one(doc, None)
                                                                        .await
                                                                        .unwrap();
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if set_sync {
                                            *sync = true;
                                            send_addr_to_server(my_addresses[0].clone()).await;
                                            let my_multiaddress: Multiaddr =
                                                my_addresses[0].parse().unwrap();
                                            let str_my_multiaddr =
                                                serde_json::to_string(&my_multiaddress).unwrap();
                                            match swarm.behaviour_mut().gossipsub.publish(
                                                clients_topic.clone(),
                                                str_my_multiaddr.as_bytes(),
                                            ) {
                                                Ok(_) => {
                                                    write_log("my address propagate to the network")
                                                }
                                                Err(e) => write_log(&format!(
                                                    "my address propagation error! {e}"
                                                )),
                                            }
                                        } else {
                                            for connected in connections.clone() {
                                                swarm
                                                    .disconnect_peer_id(connected.clone())
                                                    .unwrap();
                                            }
                                            leader.clear();
                                            connections.clear();
                                            client_topic_subscriber.clear();
                                            relay_topic_subscribers.clear();
                                            clients.clear();
                                            relays.clear();
                                            dialed_addr.clear();
                                            syncing_blocks.clear();
                                            my_addresses.clear();
                                            *sync = false;
                                            break;
                                        }
                                    }
                                    Err(_) => {
                                        write_log("syncing error in get gossip(line 283)");
                                        in_syncing = false;
                                    }
                                }
                            }
                        }
                    }
                    libp2p::gossipsub::Event::Subscribed { peer_id, topic } => {
                        send_address(
                            &topic,
                            peer_id,
                            &mut swarm,
                            relay_topic_subscribers,
                            connections,
                            clients,
                            client_topic_subscriber,
                        );
                    }
                    _ => (),
                },
                CustomBehavEvent::ReqRes(req_res) => match req_res {
                    Event::Message { message, .. } => match message {
                        libp2p::request_response::Message::Request {
                            channel, request, ..
                        } => {
                            if *sync {
                                handle_requests(
                                    request,
                                    &mut swarm,
                                    channel,
                                    wallet,
                                    leader,
                                    clients_topic.clone(),
                                    relays,
                                    clients,
                                    relay_topic.clone(),
                                    local_peer_id,
                                    db.clone(),
                                    relay_topic_subscribers,
                                    client_topic_subscriber,
                                    im_first,
                                    dialed_addr,
                                )
                                .await;
                            }
                        }
                        libp2p::request_response::Message::Response { .. } => {}
                    },
                    _ => {}
                },
            },
            _ => {}
        }
    }
}
