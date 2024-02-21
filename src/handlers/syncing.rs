use async_std::stream::StreamExt;
use mongodb::{
    bson::{doc, from_document, Document},
    Collection,
};
use std::{
    fs::{self, File},
    io::{BufReader, Write},
    process::Command,
};

use super::{
    create_log::write_log, db_connection::blockchain_db, recieved_block::create_hash,
    structures::Block,
};

pub async fn syncing(dialed_addr: String) -> Result<(), ()> {
    match blockchain_db().await {
        Ok(db) => {
            let trim_addr = dialed_addr.trim_start_matches("/ip4/");
            let split_addr = trim_addr.split("/").next();
            match split_addr {
                Some(addr) => {
                    let blockchain_addr =
                        format!("http://{}:33369/blockchain/blockchain.zip", addr);
                    let mut blockchain_output =
                        fs::File::create("/home/blockchain.zip").unwrap();

                    match reqwest::get(blockchain_addr).await {
                        Ok(res) => {
                            let mut body = res.bytes_stream();

                            //write response to blockchain.zip file
                            loop {
                                match body.next().await {
                                    Some(item) => {
                                        let chunk = item.unwrap();
                                        blockchain_output.write_all(&chunk).unwrap();
                                    }
                                    None => break,
                                }
                            }

                            //---------------------------------------------------------
                            //unzip blockchain.zip and create files from it
                            match Command::new("unzip")
                                .arg("/home/blockchain.zip")
                                .arg("-d")
                                .arg("/home/")
                                .status()
                            {
                                Ok(_) => {
                                    //---------------------------------------------------------
                                    //open and read reciepts.bson file and insert it to database
                                    let bson_file =
                                        File::open("/home/etc/dump/Blockchain/reciept.bson").unwrap();
                                    let mut reader = BufReader::new(bson_file);

                                    let reciept_coll: Collection<Document> =
                                        db.collection("reciept");
                                    reciept_coll.delete_many(doc! {}, None).await.unwrap();

                                    while let Ok(doc) = Document::from_reader(&mut reader) {
                                        reciept_coll.insert_one(doc, None).await.unwrap();
                                    }

                                    //---------------------------------------------------------
                                    //open and read UTXOs.bson file and insert it to database
                                    let bson_file =
                                        File::open("/home/etc/dump/Blockchain/UTXOs.bson").unwrap();
                                    let mut reader = BufReader::new(bson_file);

                                    let utxos_coll: Collection<Document> = db.collection("UTXOs");
                                    utxos_coll.delete_many(doc! {}, None).await.unwrap();

                                    while let Ok(doc) = Document::from_reader(&mut reader) {
                                        utxos_coll.insert_one(doc, None).await.unwrap();
                                    }

                                    //---------------------------------------------------------
                                    //open and read Blocks.bson file and insert it to database
                                    let bson_file =
                                        File::open("/home/etc/dump/Blockchain/Blocks.bson").unwrap();
                                    let mut reader = BufReader::new(bson_file);

                                    let block_coll: Collection<Document> = db.collection("Blocks");
                                    block_coll.delete_many(doc! {}, None).await.unwrap();

                                    let mut prev_hash = String::new();

                                    while let Ok(doc) = Document::from_reader(&mut reader) {
                                        let block: Block = from_document(doc.clone()).unwrap();

                                        if block.header.prevhash
                                            != "This block is Genesis".to_string()
                                            && block.header.prevhash == prev_hash
                                        {
                                            prev_hash.clear();
                                            let str_block_body =
                                                serde_json::to_string(&block.body).unwrap();
                                            prev_hash.push_str(&create_hash(str_block_body));
                                            block_coll.insert_one(doc, None).await.unwrap();
                                        } else if block.header.prevhash
                                            == "This block is Genesis".to_string()
                                        {
                                            prev_hash.clear();
                                            let str_block_body =
                                                serde_json::to_string(&block.body).unwrap();
                                            prev_hash.push_str(&create_hash(str_block_body));
                                            block_coll.insert_one(doc, None).await.unwrap();
                                        } else {
                                            return Err(());
                                        }
                                    }
                                    return Ok(());
                                }
                                Err(e) => {
                                    println!("zip error");
                                    write_log(format!("unzip blockchain.zip error:{e}"));
                                    return Err(());
                                }
                            }
                        }
                        Err(e) => {
                            write_log(format!("get blockchain.zip from a rpc sserver problem:{e}"));
                            return Err(());
                        }
                    }
                }
                None => {
                    write_log("spliting address in syncing.rs problem".to_string());
                    return Err(());
                }
            }
        }
        Err(_e) => return Err(()),
    }
}
