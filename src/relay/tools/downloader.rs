use std::{fs::File, io::Write};

use libp2p::futures::StreamExt;

use super::create_log::write_log;

pub struct Downloader;

impl Downloader {
    pub async fn download<'a>(url: &str, file_name: &str) -> Result<(), &'a str> {
        let mut output = File::create(file_name).unwrap();
        let client = reqwest::Client::new();
        match client.get(url).send().await {
            Ok(response) => {
                let file_size = response.content_length().unwrap();
                let mut file_body = response.bytes_stream();
                let mut i = 0 as f64;

                loop {
                    match file_body.next().await {
                        Some(item) => {
                            let chunk = item.unwrap();
                            output.write_all(&chunk).unwrap();
                            i += chunk.len() as f64;
                            let percent = i / (file_size as f64) * 100.0;
                            if percent.round() == 100.0 {
                                write_log(&format!("Blockchain downloading: {}", percent.round()))
                            }
                        }
                        None => break,
                    }
                }
                output.flush().unwrap();
                Ok(write_log("Blockchain downloaded."))
            }
            Err(_e) => Err("Request Problem For Download-(tools/downloader-35)"),
        }
    }
}
