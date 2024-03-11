use std::sync::Arc;
use std::sync::Mutex;

use relay_node::handle_requests;
use relay_node::new_swarm;
use relay_node::run;

#[tokio::main]
async fn main() {
    let swarm_config = new_swarm().await;
    let local_peer_id = swarm_config.1;
    let swarm = Arc::new(Mutex::new(swarm_config.0));
    let (_, _) = tokio::join!(
        run(Arc::clone(&swarm), local_peer_id),
        handle_requests()
    );
}
