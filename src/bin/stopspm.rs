use std::{fs, io::Write, net::TcpStream};

use clap::Parser;
use spmspm_pim::cli::StopCli;

fn main() {
    let cli = StopCli::parse();
    let port = cli.port.unwrap_or_else(|| {
        let path = cli.file_path.unwrap_or("port".into());
        let port = fs::read_to_string(path).unwrap().parse().unwrap();
        port
    });
    let addr = format!("127.0.0.1:{}", port);
    let mut stream = TcpStream::connect(&addr).unwrap();
    stream.write_all(&33u32.to_le_bytes()).unwrap();
}
