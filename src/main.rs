use relay_node::handle_requests;
use relay_node::run;
use relay_node::SWARM;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let (_, _) = tokio::join!(
        run(Arc::clone(&SWARM.0), SWARM.1),
        handle_requests()
    );
}
