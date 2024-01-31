use libp2p::{identity::PublicKey, PeerId};
use sha2::{Digest, Sha256};
use sp_core::Pair;

use super::{
    db_connection::blockchain_db,
    reciept::{coinbase_reciept, insert_reciept},
    structures::{Block, FullNodes, GossipMessage, NextLeader, UtxoData, UTXO},
};

use mongodb::{
    bson::{doc, from_document, to_document, Document},
    options::FindOneOptions,
    Collection,
};

//interpreter of messages.................................................................................
pub async fn verifying_block(
    str_msg: &String,
    leader: &mut String,
    fullnode_subs: &mut Vec<FullNodes>,
) {
    match serde_json::from_str::<GossipMessage>(&str_msg) {
        Ok(gossip_message) => {
            println!("fullnodes: {:#?}", fullnode_subs);
            let validator_peerid: PeerId = gossip_message.block.header.validator.parse().unwrap();
            println!("validator peer id: {}", validator_peerid);
            //check leader that is equal with curren leader in our leader or not
            let mut validate_leader = true;
            if leader.len() > 0 {
                let current_leader: PeerId = leader.parse().unwrap();
                if current_leader == validator_peerid {
                    validate_leader = true
                } else {
                    validate_leader = false
                }
            }

            if validate_leader {
                //get validator public key
                let validator_publickey = PublicKey::try_decode_protobuf(
                    &gossip_message.block.header.block_signature.public,
                );
                match validator_publickey {
                    Ok(pubkey) => {
                        //check validator peerid
                        let check_pid_with_public_key =
                            PeerId::from_public_key(&pubkey) == validator_peerid;

                        //check block signature
                        let mut verify_block_sign = false;

                        if gossip_message.block.header.prevhash != "This block is Genesis" {
                            let str_block_body_for_verify =
                                gossip_message.block.body.coinbase.tx_hash.clone();
                            for i in fullnode_subs.clone() {
                                if i.peer_id == validator_peerid {
                                    verify_block_sign = sp_core::ecdsa::Pair::verify(
                                        &gossip_message.block.header.block_signature.signature[0],
                                        str_block_body_for_verify,
                                        &i.public_key,
                                    );
                                    break;
                                }
                            }
                        } else {
                            verify_block_sign = true;
                        }

                        if check_pid_with_public_key {
                            if verify_block_sign {
                                submit_block(gossip_message, leader).await;
                            } else {
                                println!("verify block sign error!");
                            }
                        } else {
                            println!("check pid with public key error!");
                        }
                    }
                    Err(_) => {
                        println!("validator public key error!");
                    }
                }
            } else {
                println!("validate leader error!");
            }
        }
        Err(_) => {
            if let Ok(identifier) = serde_json::from_str::<NextLeader>(&str_msg) {
                if leader.len() > 0 && identifier.identifier_peer_id.to_string() == leader.clone() {
                    leader.clear();
                    leader.push_str(&identifier.next_leader.to_string());
                }
            }
        }
    }
}

//check block in database and check transactions in mempool and then instert it to database
async fn submit_block(gossip_message: GossipMessage, leader: &mut String) {
    match blockchain_db().await {
        Ok(db) => {
            let blocks_coll: Collection<Document> = db.collection("Blocks");
            let utxos_coll = db.collection("UTXOs");
            let filter = doc! {"header.blockhash": gossip_message.block.header.blockhash.clone()};
            let same_block = blocks_coll.find_one(filter, None).await.unwrap();

            let last_block_filter = doc! {"_id": -1};
            let last_block_find_opt = FindOneOptions::builder().sort(last_block_filter).build();
            let last_block_doc = blocks_coll.find_one(None, last_block_find_opt).await;

            match last_block_doc {
                Ok(doc) => {
                    match doc {
                        Some(last_block_document) => {
                            let last_block: Block = from_document(last_block_document).unwrap();

                            let block_verify =
                                check_txs(gossip_message.clone(), utxos_coll.clone()).await; //remove transaction if it is in mempool or remove from UTXOs collection if it is not in mempool

                            if block_verify {
                                match same_block {
                                    None => {
                                        if last_block.header.blockhash
                                            == gossip_message.block.header.prevhash
                                        {
                                            let new_block_doc =
                                                to_document(&gossip_message.block).unwrap();
                                            blocks_coll
                                                .insert_one(new_block_doc, None)
                                                .await
                                                .unwrap(); //insert block to DB

                                            handle_block_reward(
                                                gossip_message.clone(),
                                                utxos_coll.clone(),
                                            )
                                            .await; //insert or update node utxos for rewards and fees

                                            handle_tx_utxos(
                                                gossip_message.clone(),
                                                utxos_coll.clone(),
                                            )
                                            .await;
                                            //update utxos in database for transactions
                                            //check next leader
                                            leader.clear();
                                            leader.push_str(&gossip_message.next_leader);
                                        } else {
                                            println!("block prev hash problem!");
                                        }
                                    }
                                    Some(_) => println!("find same block!"),
                                }
                            } else {
                                println!("check trx in block verify problem!");
                            }
                        }
                        None => {
                            if gossip_message.block.header.prevhash
                                == "This block is Genesis".to_string()
                            {
                                let new_block_doc = to_document(&gossip_message.block).unwrap();
                                blocks_coll.insert_one(new_block_doc, None).await.unwrap(); //insert block to DB
                                handle_block_reward(gossip_message.clone(), utxos_coll.clone())
                                    .await;
                                handle_tx_utxos(gossip_message.clone(), utxos_coll.clone()).await;
                                //update utxos in database for transactions
                                //check next leader
                                leader.clear();
                                leader.push_str(&gossip_message.next_leader);
                                println!("rewards in none block add");
                            }
                        }
                    }
                }
                Err(_) => {
                    if gossip_message.block.header.prevhash == "This block is Genesis".to_string() {
                        let new_block_doc = to_document(&gossip_message.block).unwrap();
                        blocks_coll.insert_one(new_block_doc, None).await.unwrap(); //insert block to DB
                        handle_block_reward(gossip_message.clone(), utxos_coll.clone()).await;
                        handle_tx_utxos(gossip_message.clone(), utxos_coll.clone()).await;
                        //update utxos in database for transactions
                        //check next leader
                        leader.clear();
                        leader.push_str(&gossip_message.next_leader);
                        println!("rewards in err block add");
                    }
                }
            }
        }
        Err(_e) => {}
    }
}

async fn check_txs(gossip_message: GossipMessage, utxos_coll: Collection<Document>) -> bool {
    let mut block_verify = true;
    for tx in gossip_message.block.body.transactions.clone() {
        let signed_message = tx.tx_hash.clone();

        //create hash of tx
        let mut check_hasher = Sha256::new();
        check_hasher.update(tx.input.input_hash.clone());
        check_hasher.update(tx.output.output_hash.clone());
        let check_hash = format!("{:x}", check_hasher.finalize());

        //create hash of tx inputs
        let tx_input_str = serde_json::to_string(&tx.input.input_data).unwrap();
        let input_hash = create_hash(tx_input_str);

        //create hash of outputs
        let tx_output_str = serde_json::to_string(&tx.output.output_data).unwrap();
        let output_hash = create_hash(tx_output_str);

        //check tx signature
        let sign_verify = sp_core::ecdsa::Pair::verify(
            &tx.input.signatures[0],
            signed_message,
            &tx.output.output_data.sigenr_public_keys[0],
        );

        //check hashs
        let input_checker = tx.input.input_hash == input_hash;
        let output_checker = tx.output.output_hash == output_hash;
        let txhash_checker = tx.tx_hash == check_hash;

        if sign_verify && input_checker && output_checker && txhash_checker {
            let user_utxo_filter = doc! {"public_key": tx.output.output_data.sigenr_public_keys[0].clone().to_string()};
            let find_utxo_doc = utxos_coll
                .find_one(user_utxo_filter.clone(), None)
                .await
                .unwrap();
            if let Some(doc) = find_utxo_doc {
                let mut user_utxo: UTXO = from_document(doc).unwrap();
                for utxo in tx.input.input_data.utxos {
                    let index = user_utxo
                        .utxos
                        .iter()
                        .position(|uu| *uu.output_hash == utxo.output_hash);
                    match index {
                        Some(i) => {
                            user_utxo.utxos.remove(i);
                            let user_utxo_todoc = to_document(&user_utxo).unwrap();
                            utxos_coll
                                .replace_one(user_utxo_filter.clone(), user_utxo_todoc, None)
                                .await
                                .unwrap();
                        }
                        None => {}
                    }
                }
            }
        } else {
            block_verify = false;
        }
    }
    block_verify
}

async fn handle_block_reward(gossip_message: GossipMessage, utxos_coll: Collection<Document>) {
    coinbase_reciept(
        gossip_message.block.body.coinbase.clone(),
        Some(gossip_message.block.header.number.clone()),
        "Confirmed".to_string(),
        "Coinbase".to_string(),
        gossip_message.block.header.clone(),
    )
    .await;
    for i in gossip_message.block.body.coinbase.output.utxos.clone() {
        let cb_utxo_filter = doc! {"public_key": i.output_unspent.public_key.clone()};
        let utxo_doc = utxos_coll
            .find_one(cb_utxo_filter.clone(), None)
            .await
            .unwrap();
        let utxo = UtxoData {
            transaction_hash: gossip_message.block.body.coinbase.tx_hash.clone(),
            unspent: i.output_unspent.unspent.round_dp(12),
            output_hash: i.hash.clone(),
            block_number: gossip_message.block.header.number,
        };
        if let Some(doc) = utxo_doc {
            let mut node_utxo: UTXO = from_document(doc).unwrap();
            let mut output_hash = Vec::new();
            for i in node_utxo.utxos.clone() {
                output_hash.push(i.output_hash);
            }
            let mut exist_hash = 0;
            for hash in output_hash {
                if hash == i.hash.clone() {
                    exist_hash += 1;
                    break;
                }
            }
            if exist_hash == 0 {
                node_utxo.utxos.push(utxo);
            }
            let node_utxo_to_doc = to_document(&node_utxo).unwrap();
            utxos_coll
                .replace_one(cb_utxo_filter.clone(), node_utxo_to_doc, None)
                .await
                .unwrap();
        } else {
            let node_pub_key = i.output_unspent.public_key.clone();
            let utxo = UTXO {
                public_key: node_pub_key,
                utxos: vec![utxo],
            };
            let utxo_to_doc = to_document(&utxo).unwrap();
            utxos_coll.insert_one(utxo_to_doc, None).await.unwrap();
        }
    }
}

async fn handle_tx_utxos(gossip_message: GossipMessage, utxos_coll: Collection<Document>) {
    for tx in gossip_message.block.body.transactions.clone() {
        insert_reciept(
            tx.clone(),
            Some(gossip_message.block.header.number),
            "Confirmed".to_string(),
            "".to_string(),
        )
        .await;
        for utxo in tx.output.output_data.utxos {
            let tx_utxo_filter = doc! {"public_key": &utxo.output_unspent.public_key};
            let utxo_doc = utxos_coll
                .find_one(tx_utxo_filter.clone(), None)
                .await
                .unwrap();
            let new_utxo = UtxoData {
                transaction_hash: tx.tx_hash.clone(),
                unspent: utxo.output_unspent.unspent.round_dp(12),
                output_hash: utxo.hash.clone(),
                block_number: gossip_message.block.header.number,
            };
            if let Some(doc) = utxo_doc {
                let mut user_utxo: UTXO = from_document(doc).unwrap();
                let mut user_utxos_outout_hashs = Vec::new();
                for i in user_utxo.utxos.clone() {
                    user_utxos_outout_hashs.push(i.output_hash);
                }
                let mut exist_hash = 0;
                for hash in user_utxos_outout_hashs {
                    if hash == utxo.hash.clone() {
                        exist_hash += 1;
                        break;
                    }
                }
                if exist_hash == 0 {
                    user_utxo.utxos.push(new_utxo);
                }
                let user_utxo_doc = to_document(&user_utxo).unwrap();
                utxos_coll
                    .replace_one(tx_utxo_filter.clone(), user_utxo_doc, None)
                    .await
                    .unwrap();
            } else {
                let public_key = utxo.output_unspent.public_key.clone();
                let mut utxos = Vec::new();
                utxos.push(new_utxo);
                let new_utxo = UTXO { public_key, utxos };
                let user_utxo_doc = to_document(&new_utxo).unwrap();
                utxos_coll.insert_one(user_utxo_doc, None).await.unwrap();
            }
        }
    }
}

//generate 1 hash from a string
pub fn create_hash(data: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
