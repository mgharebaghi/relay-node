use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use axum::{
    extract::{self},
    Json,
};
use futures::StreamExt;
use libp2p::{swarm::SwarmEvent, Multiaddr};

use crate::{
    handlers::{handle_events::Listeners, structures::{Req, Transaction}},
    new_swarm, write_log,
};

use super::server::TxRes;

pub async fn handle_transaction(
    // mut tx: Extension<Sender<String>>,
    extract::Json(transaction): extract::Json<Transaction>,
) -> Json<TxRes> {
    write_log("get transaction");
    //insert transaction reciept into db
    let str_trx = serde_json::to_string(&transaction).unwrap();
    propagation(str_trx).await;

    //send response to the client
    let tx_res = TxRes {
        hash: transaction.tx_hash,
        status: "sent".to_string(),
        description: "Wait for submit".to_string(),
    };
    return Json(tx_res);
}

async fn propagation(str_trx: String) {
    write_log("in propagation");
    {
        let myaddr_file = File::open("/etc/myaddress.dat");
        match myaddr_file {
            Ok(file) => {
                write_log("in ok file open");
                let reader = BufReader::new(file);
                let mut my_addr = String::new();
                for addr in reader.lines() {
                    let address = addr.unwrap();
                    my_addr.push_str(&address);
                }
                let mut swarm = new_swarm().await.0;
                let my_multiaddr: Multiaddr = my_addr.parse().unwrap();
                let mut listeners = Listeners { id: Vec::new() };
                match swarm.dial(my_multiaddr) {
                    Ok(_) => loop {
                        match swarm.select_next_some().await {
                            SwarmEvent::NewListenAddr { listener_id, .. } => {
                                listeners.id.push(listener_id);
                            }
                            SwarmEvent::ConnectionEstablished { connection_id, peer_id, .. } => {
                                write_log("connection established");
                                let req = Req {
                                    req: str_trx
                                };
                                swarm.behaviour_mut().req_res.send_request(&peer_id, req);
                                swarm.close_connection(connection_id);
                                for listener in listeners.id {
                                    swarm.remove_listener(listener);
                                    write_log("remove listener");
                                }
                                break;
                            }
                            _ => {}
                        }
                    },
                    Err(_) => {}
                }
            }
            Err(e) => {
                write_log(&format!("myaddress file opening problem: {}", e));
            }
        }
    }
}
