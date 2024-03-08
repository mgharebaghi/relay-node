use std::sync::Arc;
use std::sync::Mutex;
use relay_node::handle_requests;
use relay_node::run;

use relay_node::new_swarm;

#[tokio::main]
async fn main() {
    let create_swarm = new_swarm().await;
    let swarm = Arc::new(Mutex::new(create_swarm.0));
    let local_peer_id = create_swarm.1;
    let (_, _) = tokio::join!(run(Arc::clone(&swarm), local_peer_id), handle_requests(Arc::clone(&swarm)));
}
