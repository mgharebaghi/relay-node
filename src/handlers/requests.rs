use super::{
    check_trx::handle_transactions,
    create_log::write_log,
    outnodes::handle_outnode,
    recieved_block::verifying_block,
    structures::{GossipMessage, Req, Res, Transaction},
    CustomBehav,
};
use libp2p::{gossipsub::IdentTopic, request_response::ResponseChannel, PeerId, Swarm};
use mongodb::{
    bson::{doc, Document},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Handshake {
    wallet: String,
    first_node: String,
}

pub async fn handle_requests(
    request: Req,
    swarm: &mut Swarm<CustomBehav>,
    channel: ResponseChannel<Res>,
    wallet: &mut String,
    leader: &mut String,
    clients_topic: IdentTopic,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
    local_peer_id: PeerId,
    db: Database,
    relay_topic_subscribers: &mut Vec<PeerId>,
    client_topic_subscriber: &mut Vec<PeerId>,
    im_first: &mut bool,
    dialed_addr: &mut Vec<String>,
) {
    if request.req == "handshake".to_string() {
        let blocks_coll: Collection<Document> = db.collection("Blocks");
        let count_docs = blocks_coll.count_documents(doc! {}).await.unwrap();
        let mut handshake_res = Handshake {
            wallet: wallet.clone(),
            first_node: String::new(),
        };

        let validators: Collection<Document> = db.collection("validators");
        let validators_count = validators.count_documents(doc! {}).await.unwrap();

        if validators_count > 0 || count_docs > 0 {
            handshake_res.first_node.push_str(&"no".to_string());
        } else {
            handshake_res.first_node.push_str(&"yes".to_string());
        }

        let str_handshake_res = serde_json::to_string(&handshake_res).unwrap();
        let response = Res {
            res: str_handshake_res,
        };
        match swarm
            .behaviour_mut()
            .req_res
            .send_response(channel, response)
        {
            Ok(_) => {}
            Err(e) => write_log(&format!("{:?}", e)),
        }
    } else if let Ok(_transaction) = serde_json::from_str::<Transaction>(&request.req) {
        println!("get transaction");
        let send_transaction = swarm
            .behaviour_mut()
            .gossipsub
            .publish(clients_topic, request.req.clone().as_bytes());
        match send_transaction {
            Ok(_) => {
                println!("transaction published to the network");
                handle_transactions(request.req, db).await; //insert transaction to db
                let response = Res {
                    res: "".to_string(),
                };
                let _ = swarm
                    .behaviour_mut()
                    .req_res
                    .send_response(channel, response);
            }
            Err(e) => {
                println!("propagate trx problem: {}", e);
                handle_transactions(request.req, db).await;
                let response = Res {
                    res: "".to_string(),
                };
                let _ = swarm
                    .behaviour_mut()
                    .req_res
                    .send_response(channel, response);
            }
        }
    } else if let Ok(gossipms) = serde_json::from_str::<GossipMessage>(&request.req) {
        println!("get request:\n{:#?}", gossipms);
        let propagation_source = gossipms.block.header.validator;
        match verifying_block(&request.req, leader, db.clone()).await {
            Ok(_) => {
                match swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(relay_topic.clone(), request.req.as_bytes())
                {
                    Ok(_) => {
                        let response = Res { res: String::new() };
                        let _ = swarm
                            .behaviour_mut()
                            .req_res
                            .send_response(channel, response);

                        //send true block to sse servers
                        let sse_topic = IdentTopic::new("sse");
                        match swarm
                            .behaviour_mut()
                            .gossipsub
                            .publish(sse_topic, request.req.clone().as_bytes())
                        {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                    Err(_) => {
                        // write_log("sending gossip message to relay topic has problem");
                    }
                }

                //send true block to connected Validators
                let validators_topic = IdentTopic::new(local_peer_id.to_string());
                match swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(validators_topic, request.req.clone().as_bytes())
                {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            Err(e) => {
                if e != "reject" {
                    handle_outnode(
                        propagation_source,
                        swarm,
                        clients_topic,
                        relays,
                        clients,
                        relay_topic,
                        leader,
                        relay_topic_subscribers,
                        client_topic_subscriber,
                        im_first,
                        dialed_addr,
                        db,
                    )
                    .await;
                    swarm.disconnect_peer_id(propagation_source).unwrap();
                }
            }
        }
    } else {
        println!("request doesn't match");
    }
}
