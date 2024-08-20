mod handlers;
mod rpc;
use handlers::practical::db::Mongodb;
use handlers::tools::create_log::write_log;
use handlers::Handler;
use middle_gossipper::check_mongo_changes::MiddleGossipper;
use middle_gossipper::middlegossiper_swarm::MiddleSwarmConf;
mod middle_gossipper;
use middle_gossipper::middlegossiper_swarm::MyBehaviour;
use rpc::Rpc;

#[tokio::main]
async fn main() {
    let mut swarm = MyBehaviour::new().await;
    match Mongodb::connect().await {
        Ok(db) => {
            let (_, _, _) = tokio::join!(
                Rpc::handle_requests(),
                MiddleGossipper::checker(&mut swarm),
                Handler::start(&db)
            );
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
