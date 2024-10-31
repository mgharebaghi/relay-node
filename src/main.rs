mod relay;
mod json_rpc;
use relay::practical::db::Mongodb;
use relay::tools::create_log::write_log;
use middle_gossipper::check_mongo_changes::MiddleGossipper;
mod middle_gossipper;
use relay::Relay;
use json_rpc::Rpc;

#[tokio::main]
async fn main() {
    match Mongodb::connect().await {
        Ok(db) => {
            let (_, _, _) = tokio::join!(
                Rpc::handle_requests(),
                MiddleGossipper::checker(&db),
                Relay::start(&db)
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
