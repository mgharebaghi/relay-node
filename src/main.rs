use std::sync::Arc;
use std::sync::Mutex;
mod handlers;
use handlers::run_relay::run;
use handlers::swarm_config::CustomBehav;
mod rpc;
use handlers::swarm_config::SwarmConf;
use middle_gossipper::middlegossiper_swarm::MiddleSwarmConf;
use rpc::handle_requests;
mod middle_gossipper;
use middle_gossipper::middlegossiper_swarm::MyBehaviour;
use middle_gossipper::check_mongo_changes::checker;

#[tokio::main]
async fn main() {
    let swarm_config = CustomBehav::new().await;
    let local_peer_id = swarm_config.1;
    let mut gossipper_swarm = MyBehaviour::new().await;
    let swarm = Arc::new(Mutex::new(swarm_config.0));
    let (_, _, _) = tokio::join!(
        run(Arc::clone(&swarm), local_peer_id),
        handle_requests(),
        checker(&mut gossipper_swarm)
    );
}
