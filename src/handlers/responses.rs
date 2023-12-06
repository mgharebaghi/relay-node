use libp2p::{PeerId, Swarm};

use super::structures::{Res, Channels, CustomBehav, ResForReq};

pub fn handle_responses(
    response: Res,
    local_peer_id: PeerId,
    channels: &mut Vec<Channels>,
    swarm: &mut Swarm<CustomBehav>,
    client_topic_subscriber: &mut Vec<PeerId>,
    relay_topic_subscribers: &mut Vec<PeerId>
) {
    println!("in reposnse");
    let mut res:ResForReq = serde_json::from_str(&response.res).unwrap();

    if res.peer.last().unwrap() == &local_peer_id {
        res.peer.pop();
        let new_res = serde_json::to_string(&res).unwrap();
        let new_response = Res { res: new_res };
        let index = channels
            .iter()
            .position(|channel| channel.peer == res.peer.last().unwrap().clone())
            .unwrap();
        if client_topic_subscriber.contains(res.peer.last().unwrap())
            || relay_topic_subscribers.contains(res.peer.last().unwrap())
        {
            swarm
                .behaviour_mut()
                .req_res
                .send_response(channels.remove(index).channel, new_response)
                .unwrap();
        }
    } else {
        println!("in sending response...");
        let index = channels
            .iter()
            .position(|channel| channel.peer == res.peer.last().unwrap().clone())
            .unwrap();
        if client_topic_subscriber.contains(res.peer.last().unwrap())
            || relay_topic_subscribers.contains(res.peer.last().unwrap())
        {
            println!("reposnse to index: {}", channels[index]);
            swarm
                .behaviour_mut()
                .req_res
                .send_response(channels.remove(index).channel, response)
                .unwrap();
        }
    }
}