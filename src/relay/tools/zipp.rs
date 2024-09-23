use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
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
        //if zip file and dump folder existed removes those at first
        let path1 = Path::new("/home/Centichain.zip");
        let path2 = Path::new("/etc/dump");

        //remove path 1
        if path1.exists() {
            std::fs::remove_file(path1).unwrap();
        }

        //remove path 2
        if path2.exists() {
            std::fs::remove_dir_all(path2).unwrap()
        }

        //after remove zip file, makes a new
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
