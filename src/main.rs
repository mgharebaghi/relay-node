use relay_node::handle_requests;
use relay_node::run;

#[tokio::main]
async fn main() {
    let (_, _) = tokio::join!(run(), handle_requests());
}
