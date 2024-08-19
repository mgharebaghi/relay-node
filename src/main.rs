mod handlers;
mod rpc;
use handlers::start;
use handlers::tools::create_log::write_log;
use handlers::tools::db::Mongodb;
use middle_gossipper::middlegossiper_swarm::MiddleSwarmConf;
use rpc::handle_requests;
mod middle_gossipper;
use middle_gossipper::check_mongo_changes::checker;
use middle_gossipper::middlegossiper_swarm::MyBehaviour;

#[tokio::main]
async fn main() {
    let mut gossipper_swarm = MyBehaviour::new().await;
    match Mongodb::connect().await {
        Ok(db) => {
            let (_, _, _) =
                tokio::join!(handle_requests(), checker(&mut gossipper_swarm), start(&db));
        }
        Err(e) => {
            write_log(&format!(
                "mongodb connection has problem! program closed.\n{}",
                e
            ));
            std::process::exit(800)
        }
    }
}
