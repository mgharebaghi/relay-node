mod handlers;
mod rpc;
use handlers::practical::db::Mongodb;
use handlers::tools::create_log::write_log;
use handlers::Handler;
use middle_gossipper::check_mongo_changes::MiddleGossipper;
mod middle_gossipper;
use rpc::Rpc;

#[tokio::main]
async fn main() {
    match Mongodb::connect().await {
        Ok(db) => {
            let (_, _, _) = tokio::join!(
                Rpc::handle_requests(),
                MiddleGossipper::checker(&db),
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
