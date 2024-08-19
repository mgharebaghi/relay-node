use futures::StreamExt;
use libp2p::{swarm::SwarmEvent, Swarm};
use mongodb::Database;

use super::swarm::CentichainBehaviour;

pub struct State;

impl State {
    pub async fn handle(swarm: &mut Swarm<CentichainBehaviour>, db: &Database) {
        'handle: loop {
            match swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("lestener address: {}", address);
                }
                _ => {}
            }
        }
    }
}
