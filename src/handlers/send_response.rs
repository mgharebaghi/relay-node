use libp2p::{Swarm, request_response::ResponseChannel};

use super::structures::{Req, CustomBehav, Res, ReqForReq, ResForReq};

pub fn send_res(request: Req, swarm: &mut Swarm<CustomBehav>, channel: ResponseChannel<Res>) {
    let response = Res {
        res: "You Are First Client".to_string(),
    };
    let original_req: ReqForReq = serde_json::from_str(&request.req).unwrap();
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