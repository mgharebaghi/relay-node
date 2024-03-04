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
            let write_file = OpenOptions::new()
                .append(true)
                .write(true)
                .open(relaylog_path)
                .unwrap();
            let mut writer = BufWriter::new(write_file);
            writeln!(writer, "{}", log).ok();
            writer.flush().unwrap();
        }
    } else {
        {
            File::create(relaylog_path).unwrap();
            let write_file = OpenOptions::new()
                .append(true)
                .write(true)
                .open(relaylog_path)
                .unwrap();
            let mut writer = BufWriter::new(write_file);
            writeln!(writer, "{}", log).ok();
            writer.flush().unwrap();
        }
    }
}
