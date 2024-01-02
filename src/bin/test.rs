use reqwest::Client;
use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize)]
struct BlockReq {
    block_number: i64,
}

#[tokio::main]
async fn main() {
    let client = Client::new();
    let req = BlockReq {
        block_number: 4
    };
    for _i in 0..2000 {
        let response = client.post("http://45.15.157.62:3390/block").json(&req).send().await.unwrap();
        println!("{:?}", response.text().await.unwrap());
    }
}