use std::{fs, io::Write, net::TcpStream};

use clap::Parser;
use spmspm_pim::cli::StopCli;

fn main() {
    let cli = StopCli::parse();
    let port = cli.port.unwrap_or_else(|| {
        let path = cli.file_path.unwrap_or_else(|| "port".into());

        fs::read_to_string(path).unwrap().parse().unwrap()
    });
    let addr = format!("127.0.0.1:{}", port);
    let mut stream = TcpStream::connect(&addr).unwrap();
    stream.write_all(&33u32.to_le_bytes()).unwrap();
}
#[cfg(test)]
mod tests {
    #[test]
    fn test_endian() {
        let array: [u8; 4] = 1u32.to_ne_bytes();
        println!("{:?}", array);
        println!("{:?}", 1u32.to_le_bytes());
        println!("{:?}", 1u32.to_be_bytes());
    }

    struct A(String, usize);
    impl Drop for A {
        fn drop(&mut self) {
            println!("drop A");
            self.1 = 0;
        }
    }
    #[derive(Debug)]
    struct B(usize, usize, usize, usize);
    #[test]
    fn test_transmute() {
        let a = A("hello".into(), 1);
        // will not drop a
        let b = unsafe { std::mem::transmute::<A, B>(a) };
        println!("{:?}", b);
    }
}
