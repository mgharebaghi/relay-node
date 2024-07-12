use std::str::FromStr;

use chrono::{SubsecRound, Utc};
// use libp2p::gossipsub::Message;
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};
use sp_core::Pair;

use super::{
    reciept::insert_reciept,
    structures::{Transaction, UTXO},
};

use mongodb::{
    bson::{doc, from_document, to_document, Document},
    Collection, Database,
};

pub async fn handle_transactions(message: String, db: Database) {
    if let Ok(mut transaction) = serde_json::from_str::<Transaction>(&message) {
        let trxs_coll: Collection<Document> = db.collection("Transactions");
        let trx_query = doc! {"tx_hash": transaction.tx_hash.clone()};
        let reciept_coll: Collection<Document> = db.collection("reciept");
        let reciept_filter = doc! {"hash": transaction.tx_hash.clone()};
        let reciept = reciept_coll.find_one(reciept_filter).await;
        match reciept {
            Ok(is) => {
                if is.is_none() {
                    transaction.fee = transaction.value * Decimal::from_str("0.01").unwrap();
                    //create hash of transaction
                    let mut check_hasher = Sha256::new();
                    check_hasher.update(transaction.input.input_hash.clone());
                    check_hasher.update(transaction.output.output_hash.clone());
                    let check_hash = format!("{:x}", check_hasher.finalize());

                    //create hash of inputs
                    let tx_input_str =
                        serde_json::to_string(&transaction.input.input_data).unwrap();
                    let mut input_hasher = Sha256::new();
                    input_hasher.update(tx_input_str);
                    let inputs_hash = format!("{:x}", input_hasher.finalize());

                    //create hash of outputs
                    let tx_output_str =
                        serde_json::to_string(&transaction.output.output_data).unwrap();
                    let mut output_hasher = Sha256::new();
                    output_hasher.update(tx_output_str.clone());
                    let output_hash = format!("{:x}", output_hasher.finalize());

                    //check transaction signature
                    let signed_message = transaction.tx_hash.clone();
                    let sign_verify = sp_core::ecdsa::Pair::verify(
                        &transaction.input.signatures[0],
                        signed_message,
                        &transaction.output.output_data.sigenr_public_keys[0],
                    );

                    //get bool as verify of hashs
                    let outputhash_check = output_hash == transaction.output.output_hash;
                    let inputhash_check = inputs_hash == transaction.input.input_hash;
                    let hash_verify = transaction.tx_hash == check_hash;

                    let mut unvalidity_num = 0 as usize;
                    for i in transaction.output.output_data.utxos.clone() {
                        if i.output_unspent.public_key
                            == transaction.output.output_data.sigenr_public_keys[0].to_string()
                        {
                            unvalidity_num += 1;
                        }
                    }
                    let trx_validity = transaction.output.output_data.utxos.len() != unvalidity_num;

                    //check if all of hashs is verify and trx_validity is legit then handle the trx
                    if trx_validity
                        && sign_verify
                        && hash_verify
                        && inputhash_check
                        && outputhash_check
                    {
                        let filter = doc! {"public_key": &transaction.output.output_data.sigenr_public_keys[0].to_string()};
                        let utxos_coll = db.collection("UTXOs");
                        let utxo_doc = utxos_coll.find_one(filter.clone()).await.unwrap();
                        if let Some(doc) = utxo_doc {
                            let mut user_utxos: UTXO = from_document(doc).unwrap();
                            let mut correct_tx = true;
                            for utxo in transaction.input.input_data.utxos.clone() {
                                let index = user_utxos
                                    .utxos
                                    .iter()
                                    .position(|u| *u.output_hash == utxo.output_hash.clone());
                                match index {
                                    Some(i) => {
                                        user_utxos.utxos.remove(i);
                                    }
                                    None => {
                                        correct_tx = false;
                                        break;
                                    }
                                }
                            }

                            if correct_tx {
                                let user_utxo_updated = to_document(&user_utxos).unwrap();
                                utxos_coll
                                    .replace_one(filter.clone(), user_utxo_updated)
                                    .await
                                    .unwrap();
                                //set fee
                                transaction.date.clear();
                                transaction
                                    .date
                                    .push_str(&Utc::now().round_subsecs(0).to_string());
                                insert_reciept(
                                    transaction.clone(),
                                    None,
                                    "pending".to_string(),
                                    "".to_string(),
                                    db,
                                )
                                .await;
                            } else {
                                insert_reciept(
                                    transaction,
                                    None,
                                    "Error".to_string(),
                                    "There is not input UTXOs".to_string(),
                                    db,
                                )
                                .await;
                                trxs_coll.delete_one(trx_query).await.unwrap();
                            }
                        } else {
                            insert_reciept(
                                transaction,
                                None,
                                "Error".to_string(),
                                "There is not input UTXOs".to_string(),
                                db,
                            )
                            .await;
                            trxs_coll.delete_one(trx_query).await.unwrap();
                        }
                    } else {
                        insert_reciept(
                            transaction,
                            None,
                            "Error".to_string(),
                            "Transaction verify problem!".to_string(),
                            db,
                        )
                        .await;
                        trxs_coll.delete_one(trx_query).await.unwrap();
                    }
                }
            }
            Err(_) => {}
        }
    }
}
