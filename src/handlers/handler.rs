use futures::StreamExt;
use libp2p::{swarm::SwarmEvent, PeerId, Swarm};
use mongodb::Database;

use super::{
    practical::{
        addresses::Listeners,
        block::Block,
        relay::{DialedRelays, Relay},
    },
    swarm::CentichainBehaviour,
    tools::create_log::write_log,
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
        let mut p2p_address = String::new();

        //start handeling of events that recieve in p2p network with relays and validators
        'handle_loop: loop {
            match swarm.select_next_some().await {
                //handle listeners and addresses
                SwarmEvent::NewListenAddr { address, .. } => {
                    if dialed_relays.is_first() {
                        //send addresses to server after generate new listener
                        //if it has error break from loop to handler(start fn)
                        if let Ok(listener) = Listeners::new(&address, peerid).await {
                            match listener.post().await {
                                Ok(_) => p2p_address.push_str(&listener.p2p),
                                Err(_) => break 'handle_loop,
                            }
                        }
                    }
                }
                //after conenction stablished check peerid and if it was in dialed relays then relay update in database
                //set peerid in database
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    write_log(&format!("Connection stablished with: {}", peer_id));
                    if let Some(relay) = dialed_relays
                        .relays
                        .iter()
                        .find(|relay| relay.addr.contains(&peer_id.to_string()))
                    {
                        match Relay::update(&mut relay.clone(), db, Some(peer_id), None).await {
                            Ok(_) => {}
                            Err(e) => write_log(e),
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
                        match relay.clone().remove(db, dialed_relays).await {
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
                _ => {}
            }
        }
    }
}
