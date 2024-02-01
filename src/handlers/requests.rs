use super::{
    check_trx::handle_transactions, create_log::write_log, send_response::send_res, structures::{Channels, CustomBehav, Req, ReqForReq, Res, Transaction}
};
use libp2p::{gossipsub::IdentTopic, request_response::ResponseChannel, PeerId, Swarm};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Handshake {
    wallet: String
}

//handle requests that recieved from clients or relays
pub async fn handle_requests(
    request: Req,
    clients: &mut Vec<PeerId>,
    swarm: &mut Swarm<CustomBehav>,
    channels: &mut Vec<Channels>,
    channel: ResponseChannel<Res>,
    relays: &mut Vec<PeerId>,
    peer: PeerId,
    local_peer_id: PeerId,
    wallet: &mut String,
    topic: IdentTopic,
) {
    if request.req.clone() == "handshake".to_string() {
        let handshake_res = Handshake {
            wallet: wallet.clone()
        };
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
        let sse_topic = IdentTopic::new("sse");
        match swarm
            .behaviour_mut()
            .gossipsub
            .publish(sse_topic, request.req.clone())
        {
            Ok(_) => {}
            Err(_) => {
                
            }
        }
        let send_transaction = swarm.behaviour_mut().gossipsub.publish(topic, request.req);
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
    } else {
        if clients.len() > 0 {
            let chnl = Channels { peer, channel };
            channels.push(chnl);

            let random_client = clients.choose(&mut rand::thread_rng()).unwrap();
            swarm
                .behaviour_mut()
                .req_res
                .send_request(random_client, request.clone());
        } else if relays.len() > 0 {
            let mut relays_without_req_sender: Vec<PeerId> = Vec::new();
            for i in 0..relays.len() {
                if relays[i] != peer {
                    relays_without_req_sender.push(relays[i].clone());
                }
            }
            if relays_without_req_sender.len() > 0 {
                let chnl = Channels { peer, channel };
                channels.push(chnl);

                let mut original_req: ReqForReq =
                    serde_json::from_str(&request.req.clone()).unwrap();
                original_req.peer.push(local_peer_id);
                let req = serde_json::to_string(&original_req).unwrap();
                let req_for_relay = Req { req };
                let random_relay = relays_without_req_sender
                    .choose(&mut rand::thread_rng())
                    .unwrap();
                // channels.push(channel);
                swarm
                    .behaviour_mut()
                    .req_res
                    .send_request(random_relay, req_for_relay);
            } else {
                send_res(request.clone(), swarm, channel);
            }
        } else {
            send_res(request.clone(), swarm, channel);
        }
    }
}
