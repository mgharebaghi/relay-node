use libp2p::{PeerId, Swarm};
use mongodb::{
    bson::{to_document, Document},
    Collection, Database,
};
use serde::{Deserialize, Serialize};

use crate::relay::{
    events::connections::ConnectionsHandler,
    practical::{leader::Leader, reciept::Reciept, swarm::CentichainBehaviour},
    tools::{create_log::write_log, syncer::Sync},
};

use super::block::Block;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockMessage {
    pub block: Block,
    pub next_leader: PeerId,
}

impl BlockMessage {
    //handle recieved block messages
    pub async fn handle<'a>(
        &self,
        swarm: &mut Swarm<CentichainBehaviour>,
        db: &'a Database,
        recvied_blocks: &mut Vec<Self>,
        sync_state: &Sync,
        last_block: &mut Vec<Block>,
        leader: &mut Leader,
        connections_handler: &mut ConnectionsHandler,
    ) -> Result<(), &'a str> {
        //if leader is true then validatig block
        if self.block.header.validator == leader.peerid.unwrap() {
            match sync_state {
                //if validator is synced then validating block
                Sync::Synced => match self.block.validation(last_block, db).await {
                    //if block was valid then insert it to blockchain
                    Ok(block) => {
                        let collection: Collection<Document> = db.collection("Blocks");
                        let doc = to_document(block).unwrap();
                        match collection.insert_one(doc).await {
                            //if block insertion doesn't problem then insert reciepts of coinbases output and transactions
                            Ok(_) => {
                                let mut is_err = None;
                                match Reciept::insertion(Some(self.block.header.number), None, Some(&self.block.body.coinbase), db).await {
                                    Ok(_) => {
                                        for transaction in self.block.body.transactions.clone() {
                                            match Reciept::confirmation(db, &transaction.hash, &self.block.header.number).await {
                                                Ok(_) => {}
                                                Err(e) => {
                                                    is_err.get_or_insert(e);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {is_err.get_or_insert(e);}
                                }
                                
                                match is_err {
                                    None => Ok(leader.update(Some(self.next_leader))), //if reciepts insertion doesn't any problem then update leader and return ok
                                    Some(e) => Err(e)
                                }
                            }
                            Err(_) => Err("Error while inserting new block to database-(generator/block/message 67)")
                        }
                    }
                    Err(e) => {
                        write_log(e);
                        leader.start_voting(db, connections_handler, swarm).await
                    }
                },
                //if validator is not synced recieved message pushs to recieved block for syncing
                Sync::NotSynced => Ok(recvied_blocks.push(self.clone())),
            }
        } else {
            connections_handler
                .remove(db, self.block.header.validator, swarm)
                .await
        }
    }
}
