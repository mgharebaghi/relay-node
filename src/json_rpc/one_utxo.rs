use std::str::FromStr;

use axum::{extract, Json};
use mongodb::{
    bson::{doc, from_document, Document},
    Collection,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::relay::{practical::db::Mongodb, tools::utxo::{Person, UTXO}};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReqBody {
    public_key: String,
    request: String,
    value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResBody {
    public_key: String,
    utxo_data: Vec<UTXO>,
    status: String,
    description: String,
}

pub async fn a_utxo(extract::Json(request): extract::Json<ReqBody>) -> Json<ResBody> {
    match Mongodb::connect().await {
        Ok(db) => {
            let utxos_coll: Collection<Document> = db.collection("UTXOs");
            let filter = doc! {"wallet": request.public_key.clone()};
            match utxos_coll.find_one(filter).await {
                Ok(doc) => match doc {
                    Some(document) => set_response_utxos(document, request),
                    None => {
                        let res = ResBody {
                            public_key: request.public_key,
                            utxo_data: Vec::new(),
                            status: "error".to_string(),
                            description: "There is no any utxo with this public key".to_string(),
                        };
                        return Json(res);
                    }
                },
                Err(_) => {
                    let res = ResBody {
                        public_key: request.public_key,
                        utxo_data: Vec::new(),
                        status: "error".to_string(),
                        description: "There is no any utxo with this public key".to_string(),
                    };
                    return Json(res);
                }
            }
        }
        Err(_) => {
            let res = ResBody {
                public_key: request.public_key,
                utxo_data: Vec::new(),
                status: "error".to_string(),
                description: "Provider Problem! please try with another providers.".to_string(),
            };
            return Json(res);
        }
    }
}

fn set_response_utxos(document: Document, request: ReqBody) -> Json<ResBody> {
    let person: Person = from_document(document).unwrap();
    let value = Decimal::from_str(&request.value).unwrap(); //convert string of requst's value to Decimal
    let fee = value * Decimal::from_str("0.01").unwrap();
    let mut all_utxos_data = Vec::new();
    let mut utxo_data = Vec::new();
    for utxo in person.utxos {
        all_utxos_data.push(utxo);
    }
    //sort utxo_data by unspent from smallest to largest
    all_utxos_data.sort_by(|a, b| a.unspent.cmp(&b.unspent));
    let unspents_sum: Decimal = all_utxos_data
        .iter()
        .map(|data| data.unspent.round_dp(12))
        .sum();
    //get nearest unespent to value for send to client
    if unspents_sum >= value + fee {
        match all_utxos_data
            .iter()
            .position(|data| data.unspent >= value + fee)
        {
            Some(index) => {
                utxo_data.push(all_utxos_data[index].clone());
            }
            None => {
                let mut sum_data = vec![all_utxos_data.pop().unwrap()];
                for i in 0..all_utxos_data.len() {
                    let sum_data_unspents_sum: Decimal =
                        sum_data.iter().map(|data| data.unspent).sum();

                    if (sum_data_unspents_sum.round_dp(12) + all_utxos_data[i].unspent.round_dp(12))
                        >= value + fee
                    {
                        for data in sum_data.clone() {
                            utxo_data.push(data);
                        }
                        utxo_data.push(all_utxos_data[i].clone());
                        break;
                    } else {
                        sum_data.push(all_utxos_data[i].clone())
                    }
                }
            }
        }

        let res = ResBody {
            public_key: request.public_key,
            utxo_data,
            status: "success".to_string(),
            description: "done".to_string(),
        };
        return Json(res);
    } else {
        let res = ResBody {
            public_key: request.public_key,
            utxo_data: Vec::new(),
            status: "error".to_string(),
            description: "You don't have enough CENTI".to_string(),
        };
        return Json(res);
    }
}
