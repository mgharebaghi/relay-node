use std::net::Ipv4Addr;
use std::process::Command;
use std::sync::{Arc, Mutex};

use libp2p::core::transport::ListenerId;
use libp2p::futures::StreamExt;
use libp2p::Multiaddr;
use libp2p::{gossipsub::IdentTopic, request_response::Event, swarm::SwarmEvent, PeerId, Swarm};
use mongodb::bson::{doc, from_document, Document};
use mongodb::options::ChangeStreamOptions;
use mongodb::Collection;

use super::create_log::write_log;
use super::db_connection::blockchain_db;
use super::get_addresses::get_addresses;
use super::gossip_messages::handle_gossip_message;
use super::handle_listeners::{handle, send_addr_to_server};
use super::outnodes::handle_outnode;
use super::reciept::insert_reciept;
use super::recieved_block::verifying_block;
use super::remove_relays::remove_peer;
use super::requests::handle_requests;
use super::send_address::send_address;
use super::structures::{
    CustomBehav, CustomBehavEvent, FullNodes, GetGossipMsg, GossipMessage, Req, Transaction,
};
use super::syncing::syncing;

#[derive(Debug)]
struct Listeners {
    id: Vec<ListenerId>,
}

pub async fn events(
    swarm: Arc<Mutex<Swarm<CustomBehav>>>,
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
    fullnodes: &mut Vec<FullNodes>,
    sync: &mut bool,
    dialed_addr: &mut Vec<String>,
    syncing_blocks: &mut Vec<GetGossipMsg>,
    im_first: bool,
) {
    handle_new_swarm_events(
        swarm,
        local_peer_id,
        my_addresses,
        clients,
        relays,
        clients_topic,
        relay_topic,
        connections,
        relay_topic_subscribers,
        client_topic_subscriber,
        wallet,
        leader,
        fullnodes,
        sync,
        dialed_addr,
        syncing_blocks,
        im_first,
    )
    .await;
}

pub async fn handle_new_trx(swarm: Arc<Mutex<Swarm<CustomBehav>>>, clients_topic: IdentTopic) {
    let mut swarm = swarm.lock().unwrap();
    let trx_coll: Collection<Document> = blockchain_db().await.unwrap().collection("Transaction");
    let pipeline = vec![doc! { "$match": { "operationType": "insert" } }];
    let option = ChangeStreamOptions::builder()
        .full_document(Some(mongodb::options::FullDocumentType::UpdateLookup))
        .build();
    let change_stream = trx_coll.watch(pipeline, option).await;
    match change_stream {
        Ok(mut stream) => {
            loop {
                match stream.next().await {
                    Some(change) => {
                        match change {
                            Ok(change_event) => {
                                if let Some(new_transaction) = change_event.full_document {
                                    let transaction: Transaction =
                                        from_document(new_transaction).unwrap();
                                    let str_trx = serde_json::to_string(&transaction).unwrap();

                                    //send true transaction to sse servers
                                    let sse_topic = IdentTopic::new("sse");
                                    match swarm
                                        .behaviour_mut()
                                        .gossipsub
                                        .publish(sse_topic, str_trx.as_bytes())
                                    {
                                        Ok(_) => {}
                                        Err(_) => {}
                                    }

                                    //send true transaction to connected Validators and relays
                                    match swarm
                                        .behaviour_mut()
                                        .gossipsub
                                        .publish(clients_topic.clone(), str_trx.as_bytes())
                                    {
                                        Ok(_) => {}
                                        Err(_) => {}
                                    }

                                    //delete transaction after sent it to the network
                                    let filter = doc! {"tx_hash": transaction.tx_hash};
                                    match trx_coll.delete_one(filter, None).await {
                                        Ok(_) => {}
                                        Err(_) => {}
                                    }
                                }
                            }
                            Err(_) => {}
                        }
                    }
                    None => {}
                }
            }
        }
        Err(e) => write_log(&e.to_string()),
    }
}

async fn handle_new_swarm_events(
    swarm: Arc<Mutex<Swarm<CustomBehav>>>,
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
    fullnodes: &mut Vec<FullNodes>,
    sync: &mut bool,
    dialed_addr: &mut Vec<String>,
    syncing_blocks: &mut Vec<GetGossipMsg>,
    im_first: bool,
) {
    let mut listeners = Listeners { id: Vec::new() };
    let mut in_syncing = false;
    let mut swarm = swarm.lock().unwrap();

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
                    fullnodes.clear();
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
                if client_topic_subscriber.contains(&peer_id) {
                    write_log(&format!("connection closed with: {}", peer_id));
                    let index = client_topic_subscriber.iter().position(|c| *c == peer_id);
                    match index {
                        Some(i) => {
                            client_topic_subscriber.remove(i);
                        }
                        None => {}
                    }
                }
                //remove from relay topic subscribers && remove from relays.dat file
                if relay_topic_subscribers.contains(&peer_id) {
                    let i_relay_subscriber = relay_topic_subscribers
                        .iter()
                        .position(|pid| *pid == peer_id);
                    match i_relay_subscriber {
                        Some(index) => {
                            write_log(&format!(
                                "rm relay topic subscriber: {}",
                                relay_topic_subscribers[index]
                            ));
                            relay_topic_subscribers.remove(index);
                            // remove_peer(peer_id).await; //remove from .dat file and send address to server for remove from relays list
                        }
                        None => {}
                    }
                }
                //remove peer from relays if it is in the relays
                match relays.iter().position(|pid| pid == &peer_id) {
                    Some(index) => {
                        write_log(&format!("remove relay: {}", relays[index]));
                        relays.remove(index);
                    }
                    None => {}
                }

                handle_outnode(
                    peer_id,
                    &mut swarm,
                    clients_topic.clone(),
                    relays,
                    clients,
                    relay_topic.clone(),
                    fullnodes,
                )
                .await;

                //remove peer from dialed address if it is in the dialed addresses
                let dialed_index = dialed_addr
                    .iter()
                    .position(|dialed| dialed.contains(&peer_id.to_string()));

                match dialed_index {
                    Some(index) => {
                        dialed_addr.remove(index);
                    }
                    None => {}
                }

                //break for dial with other relays if there is not connection with any relays
                if !im_first && relays.len() < 1 {
                    write_log("going for break in removed dialed addresses");
                    write_log(&format!(
                        "relay topic subscribers:\n{:?}",
                        relay_topic_subscribers
                    ));

                    for connected in connections.clone() {
                        swarm.disconnect_peer_id(connected.clone()).unwrap();
                    }
                    leader.clear();
                    fullnodes.clear();
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
                                fullnodes,
                            )
                            .await;
                        } else {
                            write_log("gossip message recieved while not syncing");
                            write_log(&format!("gossip message: {:?}", str_msg));
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
                            if let Ok(transaction) = serde_json::from_str::<Transaction>(&str_msg) {
                                write_log("gossip message recieved while syncing is Transaction");
                                insert_reciept(
                                    transaction,
                                    None,
                                    "pending".to_string(),
                                    "".to_string(),
                                )
                                .await;
                                write_log("reciept inserted while syncing");
                            }
                            if let Ok(addresses) = serde_json::from_str::<Vec<String>>(&str_msg) {
                                get_addresses(addresses, local_peer_id, my_addresses);
                            }
                            if str_msg == "i have a client".to_string() {
                                if connections.contains(&propagation_source)
                                    && !relays.contains(&propagation_source)
                                {
                                    relays.push(propagation_source);
                                    write_log("new relay add");
                                }
                                if !*sync && !in_syncing {
                                    in_syncing = true;
                                    let mut addr = String::new();
                                    for add in dialed_addr.clone() {
                                        if add.contains(&propagation_source.to_string()) {
                                            addr = add.clone();
                                            break;
                                        }
                                    }

                                    match syncing(addr.clone()).await {
                                        Ok(_) => {
                                            write_log("syncing completed");
                                            let fullnodes_req = Req {
                                                req: "fullnodes".to_string(),
                                            };
                                            swarm
                                                .behaviour_mut()
                                                .req_res
                                                .send_request(&propagation_source, fullnodes_req);
                                        }
                                        Err(_) => {
                                            write_log("syncing error in get gossip(line 283)");
                                            in_syncing = false;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    libp2p::gossipsub::Event::Subscribed { peer_id, topic } => send_address(
                        topic,
                        peer_id,
                        &mut swarm,
                        relay_topic_subscribers,
                        connections,
                        clients,
                        client_topic_subscriber,
                    ),
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
                                    fullnodes,
                                    leader,
                                    clients_topic.clone(),
                                    relays,
                                    clients,
                                    relay_topic.clone(),
                                    local_peer_id,
                                )
                                .await;
                            }
                        }
                        libp2p::request_response::Message::Response { response, .. } => {
                            if let Ok(fullnode_subs) =
                                serde_json::from_str::<Vec<FullNodes>>(&response.res)
                            {
                                let mut set_sync = true;
                                for fullnode in fullnode_subs.clone() {
                                    fullnodes.push(fullnode)
                                }
                                if syncing_blocks.len() > 0 {
                                    for gossipmsg in syncing_blocks.clone() {
                                        let str_msg =
                                            &serde_json::to_string(&gossipmsg.gossip).unwrap();
                                        match verifying_block(
                                            str_msg,
                                            leader,
                                            &mut fullnode_subs.clone(),
                                        )
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
                                                    let index = fullnodes.iter().position(|node| {
                                                        node.peer_id
                                                            == gossipmsg
                                                                .gossip
                                                                .block
                                                                .header
                                                                .validator
                                                                .parse()
                                                                .unwrap()
                                                    });
                                                    match index {
                                                        Some(i) => {
                                                            fullnodes.remove(i);
                                                        }
                                                        None => {}
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
                                    match swarm
                                        .behaviour_mut()
                                        .gossipsub
                                        .publish(clients_topic.clone(), str_my_multiaddr.as_bytes())
                                    {
                                        Ok(_) => write_log("my address propagate to the network"),
                                        Err(_) => write_log(
                                            "my address propagation error! handle_events(line 380)",
                                        ),
                                    }
                                } else {
                                    for connected in connections.clone() {
                                        swarm.disconnect_peer_id(connected.clone()).unwrap();
                                    }
                                    leader.clear();
                                    fullnodes.clear();
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
                        }
                    },
                    _ => (),
                },
            },
            _ => (),
        }
    }
}
