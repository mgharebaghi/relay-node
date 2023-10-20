use libp2p::{PeerId, Swarm, request_response::ResponseChannel};
use rand::seq::SliceRandom;
use super::{send_response::send_res, structures::{CustomBehav, Channels, Res, Req, ReqForReq}};

//handle requests that recieved from clients or relays
pub fn handle_requests(
    request: Req,
    clients: &mut Vec<PeerId>,
    swarm: &mut Swarm<CustomBehav>,
    channels: &mut Vec<Channels>,
    channel: ResponseChannel<Res>,
    relays: &mut Vec<PeerId>,
    peer: PeerId,
    local_peer_id: PeerId,
) {
    if clients.len() > 0 {
        let chnl = Channels { peer, channel };
        channels.push(chnl);

        let random_client = clients.choose(&mut rand::thread_rng()).unwrap();
        swarm
            .behaviour_mut()
            .req_res
            .send_request(random_client, request);
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

            let mut original_req: ReqForReq = serde_json::from_str(&request.req).unwrap();
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
            send_res(request, swarm, channel);
        }
    } else {
        send_res(request, swarm, channel);
    }
}