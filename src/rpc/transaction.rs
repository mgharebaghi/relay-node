// use std::{
//     fs::File,
//     io::{BufRead, BufReader},
// };

use axum::{
    extract::{self},
    Json,
};
// use futures::StreamExt;
// use libp2p::{
//     request_response::{Event, Message},
//     swarm::{ConnectionId, SwarmEvent},
//     Multiaddr,
// };
use mongodb::{bson::{to_document, Document}, Collection};

use crate::handlers::{
    check_trx::handle_transactions, db_connection::blockchain_db, structures::Transaction
};

use super::{
    server::TxRes,
    // swarm_cfg::{CostumBehav, CostumBehavEvent, Req, SwarmConf},
};

// struct Connection {
//     id: Vec<ConnectionId>,
// }

pub async fn handle_transaction(
    // mut tx: Extension<Sender<String>>,
    extract::Json(transaction): extract::Json<Transaction>,
) -> Json<TxRes> {
    //structure of reswponse
    let mut tx_res = TxRes {
        hash: transaction.tx_hash.clone(),
        status: String::new(),
        description: String::new(),
    };

    match blockchain_db().await {
        Ok(db) => {
            let trx_coll:Collection<Document> = db.collection("Transactions");
            let trx_doc = to_document(&transaction).unwrap();
            trx_coll.insert_one(trx_doc, None).await.unwrap();
            handle_transactions(serde_json::to_string(&transaction).unwrap()).await;
            tx_res.status = "trx sent".to_string();
        }
        Err(_) => {
            tx_res.status = "error".to_string();
            tx_res.description = "server has problem! please try with another provider.".to_string();
        }
    }

    return Json(tx_res);
}

// async fn propagation(str_trx: String, tx_res: &mut TxRes) -> &mut TxRes {
//     {
//         let myaddr_file = File::open("/etc/myaddress.dat");
//         match myaddr_file {
//             Ok(file) => {
//                 let reader = BufReader::new(file);
//                 let mut my_addr = String::new();
//                 for addr in reader.lines() {
//                     let address = addr.unwrap();
//                     my_addr.push_str(&address);
//                 }
//                 let mut swarm = CostumBehav::new().await;
//                 let my_multiaddr: Multiaddr = my_addr.parse().unwrap();
//                 let mut listeners = Listeners { id: Vec::new() };
//                 let mut connection = Connection { id: Vec::new() };
//                 match swarm.dial(my_multiaddr) {
//                     Ok(_) => loop {
//                         match swarm.select_next_some().await {
//                             SwarmEvent::NewListenAddr { listener_id, .. } => {
//                                 listeners.id.push(listener_id);
//                             }
//                             SwarmEvent::ConnectionEstablished {
//                                 connection_id,
//                                 peer_id,
//                                 ..
//                             } => {
//                                 let req = Req {
//                                     req: str_trx.clone(),
//                                 };
//                                 swarm.behaviour_mut().req_res.send_request(&peer_id, req);
//                                 connection.id.push(connection_id);
//                             }
//                             SwarmEvent::Behaviour(costume_behav) => match costume_behav {
//                                 CostumBehavEvent::ReqRes(reqres) => match reqres {
//                                     Event::Message { message, .. } => match message {
//                                         Message::Response { response, .. } => {
//                                             for conn in connection.id {
//                                                 swarm.close_connection(conn);
//                                             }
//                                             for listener in listeners.id {
//                                                 swarm.remove_listener(listener);
//                                             }
//                                             let msg = response.res;
//                                             if msg == "Your transaction sent.".to_string() {
//                                                 tx_res.status = "success".to_string();
//                                                 tx_res.description = msg;
//                                                 return tx_res;
//                                             } else {
//                                                 tx_res.status = "Error".to_string();
//                                                 tx_res.description = msg;
//                                                 return tx_res;
//                                             }
//                                         }
//                                         _ => {}
//                                     },
//                                     _ => {}
//                                 },
//                             },
//                             _ => {}
//                         }
//                     },
//                     Err(_) => {
//                         tx_res.status = "Error".to_string();
//                         tx_res.description = "dialing with a relay error".to_string();
//                         return tx_res;
//                     }
//                 }
//             }
//             Err(e) => {
//                 write_log(&format!("myaddress file opening problem: {}", e));
//                 tx_res.status = "Error".to_string();
//                 tx_res.description = "problem from server".to_string();
//                 return tx_res;
//             }
//         }
//     }
// }
