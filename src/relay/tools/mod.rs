use sha2::{Digest, Sha256};

pub mod utxo;
pub mod waiting;
pub mod create_log;
pub mod syncer;
pub mod downloader;
pub mod zipp;
pub mod bsons;
pub mod wrongdoer;
pub mod get_last_block;

pub struct MerkelRoot;

impl MerkelRoot {
    //make a merkel root from hashs of transactions
    pub fn make(transactions: Vec<&String>) -> Vec<String> {
        let mut hashs: Vec<String> = Vec::new();
        for trx in transactions {
            hashs.push(trx.to_string());
        }

        while hashs.len() > 1 {
            let left = hashs.remove(0);
            let right = hashs.remove(0);
            let mut hasher = Sha256::new();
            hasher.update(left);
            hasher.update(right);
            let root = format!("{:x}", hasher.finalize());
            hashs.push(root);
        }

        hashs
    }
}

pub struct HashMaker;

impl HashMaker {
    //make a hash from data that is an argument
    pub fn generate(data: &String) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());
        hash
    }
}
