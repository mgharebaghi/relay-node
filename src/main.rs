use async_std::stream::IntoStream;
use async_std::stream::StreamExt;
use futures::channel::mpsc;
use futures::channel::mpsc::Receiver;
use relay_node::handle_requests;
use relay_node::run;
use std::sync::Arc;
use std::sync::Mutex;

use relay_node::new_swarm;
use relay_node::write_log;
// use relay_node::Transaction;

#[tokio::main]
async fn main() {
    let create_swarm = new_swarm().await;
    let swarm = Arc::new(Mutex::new(create_swarm.0));
    let local_peer_id = create_swarm.1;
    let (tx, rx) = mpsc::channel(32);
    let mut rx_stream: Receiver<String> = rx.into_stream();
    let (_, _, _) = tokio::join!(
        run(Arc::clone(&swarm), local_peer_id),
        handle_requests(Arc::new(Mutex::new(tx))),
        get_trx(&mut rx_stream)
    );
}

async fn get_trx(rx_stream: &mut Receiver<String>) {
    while let Some(trx) = rx_stream.next().await {
        write_log(&trx);
    }
}
