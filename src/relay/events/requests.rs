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
        reciept::Reciept,
        swarm::{CentichainBehaviour, Req, Res},
        transaction::Transaction,
    },
    tools::{create_log::write_log, syncer::Sync},
};

use super::{connections::ConnectionsHandler, gossip_messages::GossipMessages};

// Enum representing different types of requests that can be handled
#[derive(Debug, Serialize, Deserialize)]
pub enum Requests {
    Handshake(String),
    BlockMessage(BlockMessage),
    Transaction(Transaction),
}

// Structure for handshake response
#[derive(Debug, Serialize)]
struct HandshakeResponse {
    wallet: String,
    first_node: FirstChecker,
}

// Enum to indicate if the node is the first in the network
#[derive(Debug, Serialize)]
enum FirstChecker {
    Yes,
    No,
}

impl HandshakeResponse {
    // Create a new HandshakeResponse with default FirstChecker as No
    fn new(wallet: String) -> Self {
        Self {
            wallet,
            first_node: FirstChecker::No,
        }
    }

    // Set the node as the first node in the network
    fn set_is_first(&mut self) {
        self.first_node = FirstChecker::Yes
    }
}

impl Requests {
    // Main handler for processing different types of requests
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
        // Parse the request and handle it based on its type
        if let Ok(request_model) = serde_json::from_str::<Self>(&request.req) {
            match request_model {
                // Handle handshake request
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

                // Handle transaction request
                Requests::Transaction(transaction) => match transaction.validate(db).await {
                    Ok(trx) => match trx.insertion(db, leader, connections_handler, swarm).await {
                        Ok(_) => {
                            // Propagate the transaction to the network
                            let gossip_message = GossipMessages::Transaction(transaction.clone());
                            let str_gossip_message =
                                serde_json::to_string(&gossip_message).unwrap();
                            match swarm
                                .behaviour_mut()
                                .gossipsub
                                .publish(IdentTopic::new("validator"), str_gossip_message)
                            {
                                Ok(_) => {
                                    // Insert receipt and send response
                                    match Reciept::insertion(None, Some(&transaction), None, db)
                                        .await
                                    {
                                        Ok(_) => {
                                            let response = Res {
                                                res: "".to_string(),
                                            };
                                            match swarm
                                                .behaviour_mut()
                                                .reqres
                                                .send_response(channel, response)
                                            {
                                                Ok(_) => {}
                                                Err(_e) => {
                                                    write_log(
                                                        "Sending transaction response error!",
                                                    );
                                                    std::process::exit(0)
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            write_log(e);
                                            std::process::exit(0)
                                        }
                                    }
                                }
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
                    Err(e) => match connections_handler.remove(db, sender, swarm).await{
                        Ok(_) => write_log(e),
                        Err(e) => write_log(e),
                    },
                },

                // Handle block message request
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
                            // Propagate the block message to the network
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

    // Handle handshake requests
    async fn handshaker<'a>(
        swarm: &mut Swarm<CentichainBehaviour>,
        db: &'a Database,
        wallet: String,
        channel: ResponseChannel<Res>,
        sender: PeerId,
        leader: &mut Leader,
    ) -> Result<(), &'a str> {
        let mut handshake_reponse = HandshakeResponse::new(wallet);
        
        // Check blocks count and validators count from DB
        let b_collection: Collection<Document> = db.collection("Blocks");
        let v_collection: Collection<Document> = db.collection("validators");
        let blocks_count = b_collection.count_documents(doc! {}).await.unwrap();
        let validators_count = v_collection.count_documents(doc! {}).await.unwrap();

        // If there are no blocks and validators, set this node as the first node
        if blocks_count == 0 && validators_count == 0 {
            handshake_reponse.set_is_first();
            leader.update(Some(sender));
        }

        // Serialize response and send it
        let str_response = serde_json::to_string(&handshake_reponse).unwrap();
        let response = Res::new(str_response);

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
