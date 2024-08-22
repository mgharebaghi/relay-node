use futures::StreamExt;
use libp2p::{gossipsub::Event, swarm::SwarmEvent, PeerId, Swarm};
use mongodb::Database;

use super::{
    practical::{
        addresses::Listeners,
        block::Block,
        connections::{ConnectionsHandler, Kind},
        relay::{DialedRelays, First},
    },
    swarm::{CentichainBehaviour, CentichainBehaviourEvent},
    tools::{create_log::write_log, syncer::Sync},
};

pub struct State;

impl State {
    pub async fn handle(
        swarm: &mut Swarm<CentichainBehaviour>,
        db: &Database,
        dialed_relays: &mut DialedRelays,
        peerid: &PeerId,
    ) {
        //Prerequisites
        let mut recieved_blocks: Vec<Block> = Vec::new();
        let mut multiaddress = String::new();
        let mut sync_state = Sync::new();
        let mut last_block: Vec<Block> = Vec::new();
        let mut connections_handler = ConnectionsHandler::new();

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
                                Ok(_) => {}
                                Err(_) => std::process::exit(0),
                            },
                            First::No => {
                                multiaddress.push_str(&address.to_string()) //must save address for after syncing that should posts it to server
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
                                write_log(&format!("Dialing failed with: {}", peer_id.unwrap()));
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
                //break to dialing(mod) if there is no any connection with atleast a relay
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    match connections_handler.remove(db, peer_id).await {
                        Ok(_) => {
                            write_log(&format!("connection closed and removed with: {}", peer_id));
                            //break to dialing if there is no any connections with relays
                            if connections_handler
                                .connections
                                .iter()
                                .filter(|c| *c.kind.as_ref().unwrap() == Kind::Relay)
                                .count()
                                == 0
                            {
                                break 'handle_loop;
                            }
                        }
                        Err(e) => {
                            write_log(e);
                        }
                    }
                }

                //handle Centichain behaviour that are gossipsub and request & response
                SwarmEvent::Behaviour(behaviuor) => match behaviuor {
                    //handle gossipsub messages and subscribers
                    CentichainBehaviourEvent::Gossipsub(event) => match event {
                        Event::Subscribed { peer_id, topic } => {
                            if topic.to_string() == "relay" {
                                connections_handler.update_connection(peer_id, Kind::Relay);
                            }
                            if topic.to_string() == "validator" {
                                connections_handler.update_connection(peer_id, Kind::Validator);
                            }
                        }
                        // Event::Message {
                        //     propagation_source,
                        //     message,
                        //     ..
                        // } => {}
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        }
    }
}
