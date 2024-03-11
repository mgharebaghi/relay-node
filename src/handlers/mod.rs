mod gossip_messages;
pub mod handle_events;
mod handle_listeners;
mod outnodes;
mod remove_relays;
mod requests;
mod send_address;
pub mod structures;
pub mod check_trx;
pub mod create_log;
pub mod db_connection;
mod get_addresses;
mod handle_messages;
mod nodes_sync_announce;
mod reciept;
mod recieved_block;
mod syncing;
pub mod swarm_config;
use swarm_config::CustomBehav;
pub mod run_relay;
pub mod listening_dialing;

use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
struct Addresses {
    addr: Vec<String>,
}