use libp2p::{request_response::ResponseChannel, Swarm};

use super::structures::{CustomBehav, Req, ReqForReq, Res, ResForReq};

pub fn send_res(request: Req, swarm: &mut Swarm<CustomBehav>, channel: ResponseChannel<Res>) {
    let response = Res {
        res: "You Are First Client".to_string(),
    };
    if let Ok(original_req) = serde_json::from_str::<ReqForReq>(&request.req) {
        let res = ResForReq {
            peer: original_req.peer,
            res: response,
        };
        let str_res = serde_json::to_string(&res).unwrap();
        let final_res = Res { res: str_res };
        swarm
            .behaviour_mut()
            .req_res
            .send_response(channel, final_res)
            .unwrap();
    }
}
