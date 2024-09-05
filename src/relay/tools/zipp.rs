use std::{
    fs::{self, File},
    io::{Read, Write},
    process::Command,
};

use super::create_log::write_log;

pub struct Zip;

impl Zip {
    pub fn extract<'a>(to: &str) -> Result<(), &'a str> {
        fs::create_dir_all(to).unwrap();
        match zip::ZipArchive::new(File::open("/home/Centichain.zip").unwrap()) {
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

    pub fn maker<'a>() -> Result<(), &'a str> {
        match Command::new("mongodump")
            .arg("--db")
            .arg("Centichain")
            .arg("--out")
            .arg("/etc/dump")
            .output()
        {
            Ok(_) => {
                match Command::new("zip")
                    .arg("-r")
                    .arg("/home/Centichain.zip")
                    .arg("/etc/dump/Centichain")
                    .output()
                {
                    Ok(_) => Ok(()),
                    Err(_) => {
                        Err("Error while making zip file from database-(relay/tools/zipp 48)")
                    }
                }
            }
            Err(_) => Err("Error while mongo dumping-(relay/tools/zipp 52)"),
        }
    }
}
