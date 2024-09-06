use libp2p::{gossipsub::IdentTopic, request_response::ResponseChannel, PeerId, Swarm};
use mongodb::{
    bson::{doc, Document},
    Collection, Database,
};
use serde::{Deserialize, Serialize};
use sp_core::ed25519::Public;

use crate::relay::{
    practical::{
        block::{block::Block, message::BlockMessage},
        leader::Leader,
        swarm::{CentichainBehaviour, Req, Res},
        transaction::Transaction,
    },
    tools::{create_log::write_log, syncer::Sync},
};

use super::{connections::ConnectionsHandler, gossip_messages::GossipMessages};

#[derive(Debug, Serialize, Deserialize)]
pub enum Requests {
    Handshake(String),
    BlockMessage(BlockMessage),
    Transaction(Transaction),
}

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
        leader: &mut Leader,
        connections_handler: &mut ConnectionsHandler,
        recieved_blocks: &mut Vec<BlockMessage>,
        sync_state: &Sync,
        last_block: &mut Vec<Block>,
        sender: PeerId,
    ) {
        //if request was handhsake then goes to handshaker
        if let Ok(request_model) = serde_json::from_str::<Self>(&request.req) {
            match request_model {
                //if request was handhsake model then goes to handshaker for make the client response
                Requests::Handshake(msg) => {
                    if msg == "handshake".to_string() {
                        match Self::handshaker(
                            swarm,
                            db,
                            wallet.to_string(),
                            channel,
                            sender,
                            leader,
                        )
                        .await
                        {
                            Ok(_) => {}
                            Err(e) => write_log(e),
                        }
                    }
                }

                //if request was transaction then validating it then propagate to the network
                //if propagating has problem process, will be exited
                //if insertion to database has problem, process will be exited
                Requests::Transaction(transaction) => match transaction.validate(db).await {
                    Ok(trx) => match trx.insertion(db, leader, connections_handler, swarm).await {
                        Ok(_) => {
                            let gossip_message = GossipMessages::Transaction(transaction);
                            let str_gossip_message =
                                serde_json::to_string(&gossip_message).unwrap();
                            match swarm
                                .behaviour_mut()
                                .gossipsub
                                .publish(IdentTopic::new("validator"), str_gossip_message)
                            {
                                Ok(_) => {}
                                Err(e) => {
                                    write_log(&format!(
                                        "Gossiping transaction problem: {}",
                                        e.to_string()
                                    ));
                                    std::process::exit(0);
                                }
                            }
                        }
                        Err(e) => {
                            write_log(e);
                            std::process::exit(0);
                        }
                    },
                    Err(e) => write_log(e),
                },

                //if request was block message then goes to validating it
                //if propagating has problem, process will be exited
                //if block message handeling has problem, process will be exited
                Requests::BlockMessage(block_message) => {
                    match block_message
                        .handle(
                            swarm,
                            db,
                            recieved_blocks,
                            sync_state,
                            last_block,
                            leader,
                            connections_handler,
                        )
                        .await
                    {
                        Ok(_) => {
                            let str_block_message = serde_json::to_string(&block_message).unwrap();
                            match swarm
                                .behaviour_mut()
                                .gossipsub
                                .publish(IdentTopic::new("validator"), str_block_message)
                            {
                                Ok(_) => {}
                                Err(e) => {
                                    write_log(&format!(
                                        "Gossiping block message problem: {}",
                                        e.to_string()
                                    ));
                                    std::process::exit(0);
                                }
                            }
                        }
                        Err(e) => {
                            write_log(e);
                            std::process::exit(0);
                        }
                    }
                }
            }
        }
    }

    //handle handshake requests
    async fn handshaker<'a>(
        swarm: &mut Swarm<CentichainBehaviour>,
        db: &'a Database,
        wallet: String,
        channel: ResponseChannel<Res>,
        sender: PeerId,
        leader: &mut Leader,
    ) -> Result<(), &'a str> {
        let mut handshake_reponse = HandshakeResponse::new(wallet);
        //check blocks count and validators count from DB
        let b_collection: Collection<Document> = db.collection("Blocks");
        let v_collection: Collection<Document> = db.collection("validators");
        let blocks_count = b_collection.count_documents(doc! {}).await.unwrap();
        let validators_count = v_collection.count_documents(doc! {}).await.unwrap();

        //if there is no any blocks and validators in the network then send response as first node for validator
        if blocks_count == 0 && validators_count == 0 {
            handshake_reponse.set_is_first();
            leader.update(Some(sender));
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
            Err(_) => Err("Sending handshake response error-(handlers/practical/requests 188)"),
        }
    }
}
