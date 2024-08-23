use libp2p::{request_response::ResponseChannel, Swarm};
use mongodb::{
    bson::{doc, Document},
    Collection, Database,
};
use serde::Serialize;
use sp_core::ed25519::Public;

use crate::handlers::{
    swarm::{CentichainBehaviour, Req, Res},
    tools::create_log::write_log,
};

pub struct Requests;

//handshake response structure
#[derive(Debug, Serialize)]
struct HandshakeResponse {
    wallet: String,
    first_node: FirstChecker,
}

//firstnode enum for handshake response that if it was yes it means the validator is first validator in the network
#[derive(Debug, Serialize)]
enum FirstChecker {
    Yes,
    No,
}

//implement new for make new handshake response
//and also set_is_first for set fist node variant of handshake respnse to yes if it was first
impl HandshakeResponse {
    fn new(wallet: String) -> Self {
        Self {
            wallet,
            first_node: FirstChecker::No,
        }
    }

    fn set_is_first(&mut self) {
        self.first_node = FirstChecker::Yes
    }
}

impl Requests {
    pub async fn handler(
        db: &Database,
        request: Req,
        channel: ResponseChannel<Res>,
        swarm: &mut Swarm<CentichainBehaviour>,
        wallet: &Public,
    ) {
        //if request was handhsake then goes to handshaker
        if request.req == "handshake".to_string() {
            println!("handshake request get");
            match Self::handshaker(swarm, db, wallet.to_string(), channel).await {
                Ok(_) => {}
                Err(e) => write_log(e),
            }
        }
    }

    //handle handshake requests
    async fn handshaker<'a>(
        swarm: &mut Swarm<CentichainBehaviour>,
        db: &'a Database,
        wallet: String,
        channel: ResponseChannel<Res>,
    ) -> Result<(), &'a str> {
        let mut handshake_reponse = HandshakeResponse::new(wallet);
        //check blocks count and validators count from DB
        let b_collection: Collection<Document> = db.collection("Blocks");
        let v_collection: Collection<Document> = db.collection("validators");
        let blocks_count = b_collection.count_documents(doc! {}).await.unwrap();
        let validators_count = v_collection.count_documents(doc! {}).await.unwrap();

        //if there is no any blocks and validators in the network then send response as first node for validator
        if blocks_count == 0 && validators_count == 0 {
            handshake_reponse.set_is_first()
        }

        //serialize response to string and generate new response
        let str_response = serde_json::to_string(&handshake_reponse).unwrap();
        let response = Res::new(str_response);

        //sending response and if there was any problems return error
        match swarm
            .behaviour_mut()
            .reqres
            .send_response(channel, response)
        {
            Ok(_) => Ok(()),
            Err(_) => Err("Sending handshake response error-(handlers/practical/requests 76)"),
        }
    }
}
