use libp2p::PeerId;
use mongodb::Database;
use serde::{Deserialize, Serialize};
use sp_core::ed25519::Public;

use crate::handlers::practical::relay::Relay;

use super::{bsons::Bson, create_log::write_log, downloader::Downloader, zipp::Zip};

#[derive(Debug, Serialize, Deserialize)]
pub struct VSync {
    relay: PeerId,
    peerid: PeerId,
    msg: String,
    wallet: Public,
}

pub struct RSync;

impl RSync {
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
    pub async fn insert_bsons<'a>(db: &'a Database) -> Result<(), &'a str> {
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
                    Ok(write_log("Blockchain inserted successfully"))
                }
            }
            Err(e) => Err(e),
        }
    }
}
