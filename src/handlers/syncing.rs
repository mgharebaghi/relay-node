use async_std::stream::StreamExt;
use mongodb::{
    bson::{doc, from_document, Document},
    options::FindOneOptions,
    Collection, Database,
};
use std::{
    fs::{self, File},
    io::{BufReader, Read, Write},
    process::Command,
};

use super::{create_log::write_log, recieved_block::create_hash, structures::Block};

pub async fn syncing(dialed_addr: String, db: Database) -> Result<(), ()> {
    let trim_addr = dialed_addr.trim_start_matches("/ip4/");
    let split_addr = trim_addr.split("/").next();
    match split_addr {
        Some(addr) => {
            let blockchain_addr = format!("http://{}:33369/blockchain/blockchain.zip", addr);
            write_log(&format!("syncing with {}", blockchain_addr));

            //---------------------------------------------------------
            //remove blockchain.zip in home if exist
            let zip_exist = fs::metadata("/home/blockchain.zip");
            if zip_exist.is_ok() {
                let rm_zip = Command::new("rm").arg("/home/blockchain.zip").status();
                write_log(&format!(
                    "remove blockechain.zip:{}",
                    rm_zip.unwrap().to_string()
                ));
            }
            let mut blockchain_output = fs::File::create("/home/blockchain.zip").unwrap();

            //---------------------------------------------------------
            //get latest version of blockchain in zip format
            match reqwest::get(blockchain_addr).await {
                Ok(res) => {
                    write_log("get response from connected relay");
                    let mut body = res.bytes_stream();

                    //write response to blockchain.zip file
                    while let Some(item) = body.next().await {
                        let chunk = item.unwrap();
                        blockchain_output.write_all(&chunk).unwrap();
                    }

                    //---------------------------------------------------------
                    //remove etc in home if exist
                    let etc_exist = fs::metadata("/home/etc");
                    if etc_exist.is_ok() {
                        let rm_etc = Command::new("rm").arg("-r").arg("/home/etc").status();
                        write_log(&format!(
                            "remove etc in home direction: {}",
                            rm_etc.unwrap().to_string()
                        ));
                    }

                    //---------------------------------------------------------
                    //unzip blockchain.zip file and create files from it
                    let blockchain_file = File::open("/home/blockchain.zip").unwrap();
                    fs::create_dir_all("/home/etc/dump/Blockchain").unwrap();
                    let mut archive = zip::ZipArchive::new(blockchain_file).unwrap();
                    for i in 0..archive.len() {
                        let mut file = archive.by_index(i).unwrap();
                        if file.is_file() {
                            let mut output =
                                fs::File::create(&format!("/home/{}", file.name())).unwrap();
                            let mut file_bytes = Vec::new();
                            file.read_to_end(&mut file_bytes).unwrap();
                            output.write_all(&file_bytes).unwrap();
                        }
                    }

                    //---------------------------------------------------------
                    //open and read reciepts.bson file and insert it to database
                    let reciept_bson =
                        File::open("/home/etc/dump/Blockchain/reciept.bson").unwrap();
                    let mut reciept_reader = BufReader::new(reciept_bson);

                    let reciept_coll: Collection<Document> = db.collection("reciept");
                    let sort = doc! {"_id": -1};
                    let option = FindOneOptions::builder().sort(sort).build();
                    match reciept_coll.find_one(None, option.clone()).await {
                        Ok(doc) => {
                            if doc.is_some() {
                                match reciept_coll.delete_many(doc! {}, None).await {
                                    Ok(_) => {
                                        write_log("delete old reciept collection");
                                        while let Ok(document) =
                                            Document::from_reader(&mut reciept_reader)
                                        {
                                            reciept_coll.insert_one(document, None).await.unwrap();
                                        }
                                        write_log("insert new reciept collection");
                                    }
                                    Err(_) => {
                                        write_log("delete reciept collection error");
                                        return Err(());
                                    }
                                }
                            } else {
                                while let Ok(doc) = Document::from_reader(&mut reciept_reader) {
                                    reciept_coll.insert_one(doc, None).await.unwrap();
                                }
                            }
                        }
                        Err(_) => {
                            while let Ok(doc) = Document::from_reader(&mut reciept_reader) {
                                reciept_coll.insert_one(doc, None).await.unwrap();
                            }
                        }
                    }

                    //---------------------------------------------------------
                    //open and read UTXOs.bson file and insert it to database
                    let utxo_bson = File::open("/home/etc/dump/Blockchain/UTXOs.bson").unwrap();
                    let mut utxo_reader = BufReader::new(utxo_bson);

                    let utxo_coll: Collection<Document> = db.collection("UTXOs");

                    match utxo_coll.find_one(None, option.clone()).await {
                        Ok(doc) => {
                            if doc.is_some() {
                                match utxo_coll.delete_many(doc! {}, None).await {
                                    Ok(_) => {
                                        write_log("delete old utxo collection");
                                        while let Ok(document) =
                                            Document::from_reader(&mut utxo_reader)
                                        {
                                            utxo_coll.insert_one(document, None).await.unwrap();
                                        }
                                        write_log("insert new utxo collection");
                                    }
                                    Err(_) => {
                                        write_log("delete utxo collection error");
                                        return Err(());
                                    }
                                }
                            } else {
                                while let Ok(document) = Document::from_reader(&mut utxo_reader) {
                                    utxo_coll.insert_one(document, None).await.unwrap();
                                }
                            }
                        }
                        Err(_) => {
                            while let Ok(document) = Document::from_reader(&mut utxo_reader) {
                                utxo_coll.insert_one(document, None).await.unwrap();
                            }
                        }
                    }

                    //---------------------------------------------------------
                    //open and read outnodes.bson file and insert it to database
                    if let Ok(outnodes_bson) = File::open("/home/etc/dump/Blockchain/outnodes.bson")
                    {
                        let mut outnode_reader = BufReader::new(outnodes_bson);

                        let outnodes_coll: Collection<Document> = db.collection("outnodes");

                        match outnodes_coll.find_one(None, option.clone()).await {
                            Ok(doc) => {
                                if doc.is_some() {
                                    match outnodes_coll.delete_many(doc! {}, None).await {
                                        Ok(_) => {
                                            write_log("delete old outnodes collection");
                                            while let Ok(document) =
                                                Document::from_reader(&mut outnode_reader)
                                            {
                                                outnodes_coll
                                                    .insert_one(document, None)
                                                    .await
                                                    .unwrap();
                                            }
                                            write_log("insert new outnodes collection");
                                        }
                                        Err(_) => {
                                            write_log("delete outnodes collection error");
                                            return Err(());
                                        }
                                    }
                                } else {
                                    while let Ok(document) =
                                        Document::from_reader(&mut outnode_reader)
                                    {
                                        outnodes_coll.insert_one(document, None).await.unwrap();
                                    }
                                }
                            }
                            Err(_) => {
                                while let Ok(document) = Document::from_reader(&mut outnode_reader)
                                {
                                    outnodes_coll.insert_one(document, None).await.unwrap();
                                }
                            }
                        }
                    }

                    //---------------------------------------------------------
                    //open and read validators.bson file and insert it to database
                    if let Ok(validators_bson) = File::open("/home/etc/dump/Blockchain/validators.bson")
                    {
                        let mut validators_reader = BufReader::new(validators_bson);

                        let validators_coll: Collection<Document> = db.collection("validators");

                        match validators_coll.find_one(None, option.clone()).await {
                            Ok(doc) => {
                                if doc.is_some() {
                                    match validators_coll.delete_many(doc! {}, None).await {
                                        Ok(_) => {
                                            write_log("delete old validators collection");
                                            while let Ok(document) =
                                                Document::from_reader(&mut validators_reader)
                                            {
                                                validators_coll
                                                    .insert_one(document, None)
                                                    .await
                                                    .unwrap();
                                            }
                                            write_log("insert new validators collection");
                                        }
                                        Err(_) => {
                                            write_log("delete validators collection error");
                                            return Err(());
                                        }
                                    }
                                } else {
                                    while let Ok(document) =
                                        Document::from_reader(&mut validators_reader)
                                    {
                                        validators_coll.insert_one(document, None).await.unwrap();
                                    }
                                }
                            }
                            Err(_) => {
                                while let Ok(document) = Document::from_reader(&mut validators_reader)
                                {
                                    validators_coll.insert_one(document, None).await.unwrap();
                                }
                            }
                        }
                    }

                    //---------------------------------------------------------
                    //open and read Transactions.bson file and insert it to database
                    if let Ok(trx_bson) = File::open("/home/etc/dump/Blockchain/Transactions.bson")
                    {
                        let mut trx_reader = BufReader::new(trx_bson);

                        let trx_coll: Collection<Document> = db.collection("Transactions");

                        match trx_coll.find_one(None, option.clone()).await {
                            Ok(doc) => {
                                if doc.is_some() {
                                    match trx_coll.delete_many(doc! {}, None).await {
                                        Ok(_) => {
                                            write_log("delete old Transactions collection");
                                            while let Ok(document) =
                                                Document::from_reader(&mut trx_reader)
                                            {
                                                trx_coll.insert_one(document, None).await.unwrap();
                                            }
                                            write_log("insert new Transactions collection");
                                        }
                                        Err(_) => {
                                            write_log("delete Transactions collection error");
                                            return Err(());
                                        }
                                    }
                                } else {
                                    while let Ok(document) = Document::from_reader(&mut trx_reader)
                                    {
                                        trx_coll.insert_one(document, None).await.unwrap();
                                    }
                                }
                            }
                            Err(_) => {
                                while let Ok(document) = Document::from_reader(&mut trx_reader) {
                                    trx_coll.insert_one(document, None).await.unwrap();
                                }
                            }
                        }
                    }

                    //---------------------------------------------------------
                    //open and read Blocks.bson file and insert it to database
                    let blocks_bson = File::open("/home/etc/dump/Blockchain/Blocks.bson").unwrap();
                    let blocks_reader = BufReader::new(blocks_bson);

                    let block_coll: Collection<Document> = db.collection("Blocks");
                    match block_coll.find_one(None, option.clone()).await {
                        Ok(doc) => {
                            if doc.is_some() {
                                match block_coll.delete_many(doc! {}, None).await {
                                    Ok(_) => {
                                        write_log("delete old block collection");
                                        match insert_blocks(blocks_reader, block_coll).await {
                                            Ok(_) => {
                                                write_log("insert new block collection");
                                            }
                                            Err(_) => {
                                                write_log("insert block collection error");
                                                return Err(());
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        write_log("delete block collection error");
                                        return Err(());
                                    }
                                }
                            } else {
                                match insert_blocks(blocks_reader, block_coll).await {
                                    Ok(_) => {}
                                    Err(_) => {
                                        write_log("insert block collection error");
                                        return Err(());
                                    }
                                }
                            }
                        }
                        Err(_) => match insert_blocks(blocks_reader, block_coll).await {
                            Ok(_) => {}
                            Err(_) => {
                                write_log("insert block collection error");
                                return Err(());
                            }
                        },
                    }

                    return Ok(());
                }
                Err(e) => {
                    write_log(&format!(
                        "get blockchain.zip from a rpc sserver problem:{e}"
                    ));
                    return Err(());
                }
            }
        }
        None => {
            write_log("spliting address in syncing.rs problem");
            return Err(());
        }
    }
}

//insert blocks of blockchain.zip that recieved from rpc server
async fn insert_blocks(
    mut blocks_reader: BufReader<File>,
    block_coll: Collection<Document>,
) -> Result<(), ()> {
    let mut prev_hash = String::new();

    while let Ok(doc) = Document::from_reader(&mut blocks_reader) {
        let block: Block = from_document(doc.clone()).unwrap();

        if block.header.prevhash != "This block is Genesis".to_string()
            && block.header.prevhash == prev_hash
        {
            prev_hash.clear();
            let str_block_body = serde_json::to_string(&block.body).unwrap();
            prev_hash.push_str(&create_hash(str_block_body));
            block_coll.insert_one(doc, None).await.unwrap();
        } else if block.header.prevhash == "This block is Genesis".to_string() {
            prev_hash.clear();
            let str_block_body = serde_json::to_string(&block.body).unwrap();
            prev_hash.push_str(&create_hash(str_block_body));
            block_coll.insert_one(doc, None).await.unwrap();
        } else {
            return Err(());
        }
    }
    Ok(())
}
