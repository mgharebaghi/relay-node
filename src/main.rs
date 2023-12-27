
use relay_node::run;
use relay_node::rpc::handle_requests;

#[tokio::main]
async fn main() {
    // let x =  String::new();
    let rpc_handler = tokio::spawn(handle_requests());
    let p2p_connections = tokio::spawn(run());
    let (_, _) = tokio::try_join!(rpc_handler, p2p_connections).unwrap();
}