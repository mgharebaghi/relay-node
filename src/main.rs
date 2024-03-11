use std::sync::Arc;
use std::sync::Mutex;
mod handlers;
use handlers::run_relay::run;
use handlers::swarm_config::CustomBehav;
mod rpc;
use handlers::swarm_config::SwarmConf;
use rpc::handle_requests;

#[tokio::main]
async fn main() {
    let swarm_config = CustomBehav::new().await;
    let local_peer_id = swarm_config.1;
    let swarm = Arc::new(Mutex::new(swarm_config.0));
    let (_, _) = tokio::join!(
        run(Arc::clone(&swarm), local_peer_id),
        handle_requests()
    );
}
