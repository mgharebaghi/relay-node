use std::{
    env::consts::OS,
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
};

pub fn write_log(log: &str) {
    let mut relaylog_path = "";
    if OS == "windows" {
        relaylog_path = "relaylog.dat"
    } else if OS == "linux" {
        relaylog_path = "/etc/relaylog.dat"
    }
    let log_exist = fs::metadata(relaylog_path).is_ok();
    if log_exist {
        {
            if let Ok(write_file) = OpenOptions::new()
                .append(true)
                .write(true)
                .open(relaylog_path)
            {
                let mut writer = BufWriter::new(write_file);
                writeln!(writer, "{}", log).ok();
                match writer.flush() {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }
    } else {
        {
            match File::create(relaylog_path) {
                Ok(_) => {
                    if let Ok(write_file) = OpenOptions::new()
                        .append(true)
                        .write(true)
                        .open(relaylog_path)
                    {
                        let mut writer = BufWriter::new(write_file);
                        writeln!(writer, "{}", log).ok();
                        match writer.flush() {
                            Ok(_) => {}
                            Err(_) => {}
                        }
                    }
                }
                Err(_) => {}
            }
        }
    }
}
