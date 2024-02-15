use super::{
    check_trx::handle_transactions,
    create_log::write_log,
    outnodes::handle_outnode,
    recieved_block::verifying_block,
    structures::{CustomBehav, FullNodes, GossipMessage, Req, Res, Transaction},
};
use libp2p::{gossipsub::IdentTopic, request_response::ResponseChannel, PeerId, Swarm};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Handshake {
    wallet: String,
    first_node: String,
}

//handle requests that recieved from clients or relays
pub async fn handle_requests(
    request: Req,
    swarm: &mut Swarm<CustomBehav>,
    channel: ResponseChannel<Res>,
    wallet: &mut String,
    fullnode_subs: &mut Vec<FullNodes>,
    leader: &mut String,
    clients_topic: IdentTopic,
    relays: &mut Vec<PeerId>,
    clients: &mut Vec<PeerId>,
    relay_topic: IdentTopic,
    local_peer_id: PeerId,
) {
    if request.req == "handshake".to_string() {
        let mut handshake_res = Handshake {
            wallet: wallet.clone(),
            first_node: String::new(),
        };

        if fullnode_subs.len() > 0 {
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
            Err(e) => write_log(format!("{:?}", e)),
        }
    } else if let Ok(_transaction) = serde_json::from_str::<Transaction>(&request.req.clone()) {
        handle_transactions(request.req.clone()).await;
        // let sse_topic = IdentTopic::new("sse");
        // match swarm
        //     .behaviour_mut()
        //     .gossipsub
        //     .publish(sse_topic, request.req.clone())
        // {
        //     Ok(_) => {}
        //     Err(_) => {}
        // }
        let send_transaction = swarm
            .behaviour_mut()
            .gossipsub
            .publish(clients_topic, request.req);
        match send_transaction {
            Ok(_) => {
                let response = Res {
                    res: "Your transaction sent.".to_string(),
                };
                let _ = swarm
                    .behaviour_mut()
                    .req_res
                    .send_response(channel, response);
            }
            Err(_) => {
                let response = Res {
                    res: "sending error!".to_string(),
                };
                let _ = swarm
                    .behaviour_mut()
                    .req_res
                    .send_response(channel, response);
            }
        }
    } else if request.req.clone() == "fullnodes".to_string() {
        let str_fullnodes = serde_json::to_string(&fullnode_subs).unwrap();
        let response = Res { res: str_fullnodes };
        let _ = swarm
            .behaviour_mut()
            .req_res
            .send_response(channel, response);
    } else if let Ok(gossipms) = serde_json::from_str::<GossipMessage>(&request.req.clone()) {
        let propagation_source: PeerId = gossipms.block.header.validator.parse().unwrap();
        match verifying_block(&request.req, leader, fullnode_subs).await {
            Ok(_) => {
                match swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(relay_topic.clone(), request.req.as_bytes())
                {
                    Ok(_) => {
                        println!("block sent to {} topic", relay_topic);
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
                        fullnode_subs,
                    )
                    .await;
                    swarm.disconnect_peer_id(propagation_source).unwrap();
                }
            }
        }
    }
}
