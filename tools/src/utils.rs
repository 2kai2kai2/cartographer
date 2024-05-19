use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use std::{fs::File, io::Read};

pub fn from_cp1252<T: Read>(buffer: T) -> Result<String, std::io::Error> {
    let mut text = "".to_string();
    DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252))
        .build(buffer)
        .read_to_string(&mut text)?;
    return Ok(text);
}

pub fn read_cp1252(path: &str) -> Result<String, std::io::Error> {
    return from_cp1252(File::open(path)?);
}
