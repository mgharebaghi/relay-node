use libp2p::PeerId;
use mongodb::{
    bson::{doc, from_document, Document},
    options::FindOneOptions,
    Collection, Database,
};
use serde::{Deserialize, Serialize};
use sp_core::ed25519::Public;

use crate::relay::practical::{block::block::Block, relay::Relay};

use super::{bsons::Bson, create_log::write_log, downloader::Downloader, zipp::Zip};

#[derive(Debug, Serialize, Deserialize)]
pub struct VSync {
    relay: PeerId,
    peerid: PeerId,
    msg: String,
    wallet: Public,
}

pub struct Syncer;

impl Syncer {
    //download and extract the blockchain and then insert it to database
    async fn get_blockchain<'a>(db: &'a Database) -> Result<(), &'a str> {
        //get connected relay ip and make blockchain download link from it
        let relay_ip = Relay::ip_adress(db).await;
        match relay_ip {
            Ok(splited_addr) => {
                //start downloading blockchain from connected relay
                let url = format!("http://{}:33369/blockchain/blockchain.zip", splited_addr);
                match Downloader::download(&url, "/home/Downloads/blockchain.zip").await {
                    Ok(_) => match Zip::extract("./etc/dump/Blockchain") {
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    },
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    }

    //after getting blockchain and unzip it to bson files syncing start inserting thos into database
    async fn insert_bsons<'a>(db: &'a Database) -> Result<(), &'a str> {
        //define collection names for use in collection name parameter of bson::add
        //and us it for bson addreess in bson::add
        let collections = vec![
            "Blocks",
            "outnodes",
            "Transactions",
            "UTXOs",
            "validators, reciept, Transactions",
        ];

        let mut error = None; //for get error in loop of bson::add and return it at the end if it's some

        //get blockchain and unzip it at first and if there was problem return error of propblem
        match Self::get_blockchain(db).await {
            Ok(_) => {
                //add bsons to database or mempool by a loop
                for coll in collections {
                    let bson = format!("{}.bson", coll);

                    match Bson::add(db, &coll, &bson).await {
                        Ok(_) => {}
                        Err(e) => {
                            error = Some(e);
                            break;
                        }
                    }
                }

                //if error was some return it and if not continues syncing
                if error.is_some() {
                    Err(error.unwrap())
                } else {
                    Ok(write_log("Blockchain inserted to mongodb successfully."))
                }
            }
            Err(e) => Err(e),
        }
    }

    //after insert bsons add recieved blocks to syncing complete
    pub async fn syncing<'a>(
        db: &'a Database,
        recieved_blocks: &mut Vec<Block>,
        last_block: &mut Vec<Block>,
    ) -> Result<(), &'a str> {
        match Self::insert_bsons(db).await {
            Ok(_) => {
                let mut is_err = None;
                //finding last block after inserted bsons
                let collection: Collection<Document> = db.collection("Blocks");
                let options = FindOneOptions::builder()
                    .sort(doc! {"header.number": -1})
                    .build();
                let last_block_doc = collection
                    .find_one(doc! {})
                    .with_options(options)
                    .await
                    .unwrap()
                    .unwrap();
                //pushing last block
                let deserialized_block_doc: Block = from_document(last_block_doc).unwrap();
                last_block.clear();
                last_block.push(deserialized_block_doc);
                for block in recieved_blocks {
                    match block.validation(last_block, db).await {
                        Ok(_) => {}
                        Err(e) => {
                            is_err.get_or_insert(e);
                        }
                    }
                }
                if is_err.is_none() {
                    Ok(())
                } else {
                    Err(is_err.unwrap())
                }
            }
            Err(e) => Err(e),
        }
    }
}

//define sync for checks that relay is sync or not
#[derive(Debug, PartialEq)]
pub enum Sync {
    Synced,
    NotSynced,
}

impl Sync {
    pub fn new() -> Self {
        Self::NotSynced
    }

    pub fn synced(&mut self) {
        write_log("Relay Syncing Completed. :)");
        *self = Self::Synced;
    }
}
