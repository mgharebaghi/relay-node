use std::{
    fs::{self, File},
    io::{Read, Write},
};

use super::create_log::write_log;

pub struct Zip;

impl Zip {
    pub fn extract<'a>(to: &str) -> Result<(), &'a str> {
        fs::create_dir_all(to).unwrap();
        match zip::ZipArchive::new(File::open("/home/Downloads/blockchain.zip").unwrap()) {
            Ok(mut archive) => {
                for i in 0..archive.len() {
                    let mut item = archive.by_index(i).unwrap();
                    if item.is_file() {
                        let mut output = File::create(item.name()).unwrap();
                        let mut bytes = Vec::new();
                        item.read_to_end(&mut bytes).unwrap();
                        output.write_all(&bytes).unwrap();
                    }
                }
                Ok(write_log("Zip File Of Blockchain Extracted"))
            }
            Err(_e) => Err("Extract Zip File Error-(event/syncing-26)"),
        }
    }
}
