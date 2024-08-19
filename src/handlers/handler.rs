use futures::StreamExt;
use libp2p::{swarm::SwarmEvent, Swarm};
use mongodb::Database;

use super::{
    swarm::CentichainBehaviour,
    tools::{
        create_log::write_log,
        relay::{Relay, RelayNumber},
    },
};

pub struct State;

impl State {
    pub async fn handle(
        swarm: &mut Swarm<CentichainBehaviour>,
        db: &Database,
        relay_number: &mut RelayNumber,
    ) {
        'handle: loop {
            match swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("lestener address: {}", address);
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    write_log(&format!("Connection stablished with: {}", peer_id));
                    if let Some(relay) = relay_number
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
                SwarmEvent::OutgoingConnectionError { peer_id, .. } => {
                    if let Some(relay) = relay_number.relays.iter().find(|r| r.peerid.unwrap() == peer_id.unwrap()).cloned() {
                        match relay.remove(db, relay_number).await {
                            Ok(_) => {
                                write_log(&format!("Dialing failed with: {}", peer_id.unwrap()));
                                write_log(&format!("Removed: {}", peer_id.unwrap()));
                                if relay_number.relays.len() < 1 {
                                    break 'handle;
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
