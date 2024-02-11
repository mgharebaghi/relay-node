use async_std::stream::StreamExt;
use mongodb::{
    bson::{doc, from_document, Document},
    Collection,
};
use std::{
    fs,
    io::{BufReader, Write},
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
                    let blocks_addr = format!("http://{}:33369/blockchain/Blocks.bson", addr);
                    let utxos_addr = format!("http://{}:33369/blockchain/UTXOs.bson", addr);
                    let reciepts_addr = format!("http://{}:33369/blockchain/reciept.bson", addr);
                    let mut blocks_output = fs::File::create("/etc/Blocks.bson").unwrap();
                    let mut utxos_output = fs::File::create("/etc/UTXOs.bson").unwrap();
                    let mut reciepts_output = fs::File::create("/etc/reciept.bson").unwrap();

                    let blocks_response = reqwest::get(blocks_addr).await;
                    let utxos_response = reqwest::get(utxos_addr).await;
                    let reciepts_response = reqwest::get(reciepts_addr).await;

                    //get reciepts from server and insert it to db
                    match reciepts_response {
                        Ok(res) => {
                            let mut body = res.bytes_stream();

                            loop {
                                match body.next().await {
                                    Some(item) => {
                                        let chunk = item.unwrap();
                                        reciepts_output.write_all(&chunk).unwrap();
                                    }
                                    None => break,
                                }
                            }

                            let bson_file = fs::File::open("/etc/reciept.bson").unwrap();
                            let mut reader = BufReader::new(bson_file);

                            let reciept_coll: Collection<Document> = db.collection("reciept");
                            reciept_coll.delete_many(doc! {}, None).await.unwrap();

                            while let Ok(doc) = Document::from_reader(&mut reader) {
                                reciept_coll.insert_one(doc, None).await.unwrap();
                            }
                        }
                        Err(_) => {
                            write_log("insert reciept problem and change connection!".to_string());
                            return Err(());
                        }
                    }

                    //get utxos from server and insert it to db
                    match utxos_response {
                        Ok(res) => {
                            let mut body = res.bytes_stream();

                            loop {
                                match body.next().await {
                                    Some(item) => {
                                        let chunk = item.unwrap();
                                        utxos_output.write_all(&chunk).unwrap();
                                    }
                                    None => break,
                                }
                            }

                            let bson_file = fs::File::open("/etc/UTXOs.bson").unwrap();
                            let mut reader = BufReader::new(bson_file);

                            let utxos_coll: Collection<Document> = db.collection("UTXOs");
                            utxos_coll.delete_many(doc! {}, None).await.unwrap();

                            while let Ok(doc) = Document::from_reader(&mut reader) {
                                utxos_coll.insert_one(doc, None).await.unwrap();
                            }
                        }
                        Err(_) => {
                            write_log("insert utxos problem and change connection!".to_string());
                            return Err(());
                        }
                    }

                    //get blocks from server and insert it to db if it is correct blockchain
                    match blocks_response {
                        Ok(res) => {
                            let mut body = res.bytes_stream();

                            loop {
                                match body.next().await {
                                    Some(item) => {
                                        let chunk = item.unwrap();
                                        blocks_output.write_all(&chunk).unwrap();
                                    }
                                    None => break,
                                }
                            }

                            let bson_file = fs::File::open("/etc/Blocks.bson").unwrap();
                            let mut reader = BufReader::new(bson_file);

                            let block_coll: Collection<Document> = db.collection("Blocks");
                            block_coll.delete_many(doc! {}, None).await.unwrap();

                            let mut prev_hash = String::new();

                            while let Ok(doc) = Document::from_reader(&mut reader) {
                                let block: Block = from_document(doc.clone()).unwrap();

                                if block.header.prevhash != "This block is Genesis".to_string()
                                    && block.header.prevhash == prev_hash
                                {
                                    prev_hash.clear();
                                    let str_block_body =
                                        serde_json::to_string(&block.body).unwrap();
                                    prev_hash.push_str(&&create_hash(str_block_body));
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
                        }
                        Err(_) => {
                            return Err(());
                        }
                    }
                }
                None => {
                    write_log("spliting address in syncing.rs problem".to_string());
                    return Err(());
                }
            }

            Ok(())
        }
        Err(_e) => return Err(()),
    }
}
