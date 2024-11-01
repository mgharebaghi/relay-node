use futures::StreamExt;
use libp2p::{
    gossipsub::Event as GossipsubEvent, request_response::Event as ReqResEvent, swarm::SwarmEvent,
    PeerId, Swarm,
};
use mongodb::Database;
use sp_core::ed25519::Public;

use crate::relay::{
    events::{
        addresses::Listeners,
        connections::{ConnectionsHandler, Kind},
        gossip_messages::GossipMessages,
        requests::Requests,
    },
    practical::{
        block::{block::Block, message::BlockMessage},
        leader::Leader,
        relay::{DialedRelays, First},
        swarm::{CentichainBehaviour, CentichainBehaviourEvent},
    },
    tools::{create_log::write_log, get_last_block::LastBlock, syncer::Sync},
};

pub struct State;

impl State {
    pub async fn handle(
        swarm: &mut Swarm<CentichainBehaviour>,
        db: &Database,
        dialed_relays: &mut DialedRelays,
        peerid: &PeerId,
        wallet: &Public,
    ) {
        //Prerequisites
        let mut recieved_blocks: Vec<BlockMessage> = Vec::new();
        let mut multiaddress = String::new();
        let mut sync_state = Sync::new();
        let mut last_block: Vec<Block> = Vec::new();
        let mut connections_handler = ConnectionsHandler::new();
        let mut leader = Leader::new(None);

        //fill last block at first
        match LastBlock::get(db).await {
            Ok(is_block) => {
                if is_block.is_some() {
                    last_block.push(is_block.unwrap());
                }

                //start handeling of events that recieve in p2p network with relays and validators
                'handle_loop: loop {
                    match swarm.select_next_some().await {
                        //handle listeners and addresses
                        SwarmEvent::NewListenAddr { address, .. } => {
                            //send addresses to server after generate new listener
                            //if it has error break from loop to handler(start fn)
                            if let Ok(listener) = Listeners::new(&address, peerid, db).await {
                                match dialed_relays.first {
                                    First::Yes => match listener.post().await {
                                        Ok(_) => {
                                            sync_state.synced();
                                        }
                                        Err(_) => std::process::exit(0),
                                    },
                                    First::No => {
                                        multiaddress.push_str(&address.to_string())
                                        //must save address for after syncing that should posts it to server
                                    }
                                }
                            }
                        }

                        //after conenction stablished check peerid and if it was in dialed relays then relay update in database
                        //set peerid in database
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            match ConnectionsHandler::update_and_sync(
                                &mut connections_handler,
                                dialed_relays,
                                peer_id,
                                db,
                                &mut sync_state,
                                &mut recieved_blocks,
                                &multiaddress,
                                peerid,
                                &mut last_block,
                                &mut leader,
                            )
                            .await
                            {
                                Ok(_) => {}
                                Err(e) => {
                                    write_log(e);
                                    std::process::exit(0);
                                }
                            }
                        }

                        //handle failed dialing and remove faled dialing address from database
                        //if dialed_address was 0 then loop breaks to start new dialing *(remember if connected with another relays that they dialing to my own relay then cancel breaks)*
                        SwarmEvent::OutgoingConnectionError { peer_id, .. } => {
                            if let Some(relay) = dialed_relays
                                .relays
                                .iter()
                                .find(|r| r.addr.contains(&peer_id.unwrap().to_string()))
                            {
                                match relay.clone().delete_req(dialed_relays).await {
                                    Ok(_) => {
                                        write_log(&format!(
                                            "Dialing failed with: {}",
                                            peer_id.unwrap()
                                        ));
                                        write_log(&format!("Relay Removed: {}", peer_id.unwrap()));
                                        if dialed_relays.relays.len() < 1 {
                                            break 'handle_loop;
                                        }
                                    }
                                    Err(e) => write_log(e),
                                }
                            }
                        }

                        //handle closed connection
                        //remove closed connection from database as relay or validator
                        //break to dialing(mod) if there is no connection with atleast a relay
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            if leader.peerid.is_some() && peer_id == leader.peerid.unwrap() {
                                match leader
                                    .start_voting(db, &mut connections_handler, swarm)
                                    .await
                                {
                                    Ok(_) => {}
                                    Err(e) => write_log(e),
                                }
                            } else {
                                match connections_handler.remove(db, peer_id, swarm).await {
                                    Ok(_) => {
                                        if connections_handler.breaker(dialed_relays) {
                                            break 'handle_loop;
                                        }
                                    }
                                    Err(e) => {
                                        write_log(e);
                                    }
                                }
                            }
                        }

                        //handle Centichain behaviour that are gossipsub and request & response
                        SwarmEvent::Behaviour(behaviuor) => match behaviuor {
                            //handle requests that are handshaking, transactions and blocks
                            CentichainBehaviourEvent::Reqres(event) => match event {
                                ReqResEvent::Message { message, peer } => match message {
                                    libp2p::request_response::Message::Request {
                                        request,
                                        channel,
                                        ..
                                    } => {
                                        //if relay synced then handle requests
                                        if sync_state == Sync::Synced {
                                            Requests::handler(
                                                db,
                                                request,
                                                channel,
                                                swarm,
                                                wallet,
                                                &mut leader,
                                                &mut connections_handler,
                                                &mut recieved_blocks,
                                                &sync_state,
                                                &mut last_block,
                                                peer,
                                            )
                                            .await;
                                        }
                                    }
                                    _ => {}
                                },
                                _ => {}
                            },

                            //handle gossipsub messages and subscribers
                            CentichainBehaviourEvent::Gossipsub(event) => match event {
                                //get new subsctiber and push it to connections if there was any connections
                                GossipsubEvent::Subscribed { peer_id, topic } => {
                                    if topic.to_string() == "relay".to_string() {
                                        connections_handler.update_connection(peer_id, Kind::Relay);
                                    }
                                    if topic.to_string() == "validator".to_string() {
                                        connections_handler
                                            .update_connection(peer_id, Kind::Validator);
                                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                                    }
                                }

                                //handle messages and if it was new block message goes to handle it
                                //else if it was transaction goes to handle it
                                GossipsubEvent::Message {
                                    message,
                                    propagation_source,
                                    ..
                                } => {
                                    match GossipMessages::handle(
                                        message.data,
                                        propagation_source,
                                        db,
                                        swarm,
                                        &mut connections_handler,
                                        &mut leader,
                                        &sync_state,
                                        &mut recieved_blocks,
                                        &mut last_block,
                                    )
                                    .await
                                    {
                                        Ok(_) => {}
                                        Err(e) => {
                                            write_log(e);
                                            std::process::exit(0)
                                        }
                                    }
                                }
                                _ => {}
                            },
                        },
                        _ => {}
                    }
                }
            }
            Err(e) => write_log(e),
        }
    }
}
