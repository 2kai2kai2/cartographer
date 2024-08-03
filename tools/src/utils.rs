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

pub fn stdin_line() -> std::io::Result<String> {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    return Ok(line);
}

pub fn lines_without_comments<'a>(input: &'a str) -> impl Iterator<Item = &'a str> {
    return input
        .lines()
        .map(|line| line.split('#').next().unwrap_or(line));
}
