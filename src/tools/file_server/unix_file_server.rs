use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
};

pub struct FileServer {
    reader: BufReader<File>,
}
pub fn file_reader(file_name: &str) -> eyre::Result<FileServer> {
    let file = File::open(file_name)?;
    let reader = BufReader::new(file);
    Ok(FileServer { reader })
}

impl Read for FileServer {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}

impl BufRead for FileServer {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.reader.fill_buf()
    }
    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt)
    }
}
