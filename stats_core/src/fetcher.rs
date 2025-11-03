use std::io::{Cursor, Read};

use anyhow::Context;
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use futures::TryFutureExt;

pub fn from_cp1252<T: Read>(buffer: T) -> Result<String, std::io::Error> {
    let mut text = "".to_string();
    DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252))
        .build(buffer)
        .read_to_string(&mut text)?;
    return Ok(text);
}

/// A unified interface for fetching assets whether they are local or web-based
///
/// Relative to a base directory, typically where path `/` on inputs refers to `cartographer/cartographer_web/resources/`
pub trait Fetcher {
    fn get(&self, path: &str) -> impl std::future::Future<Output = anyhow::Result<Vec<u8>>>;

    fn get_utf8(&self, path: &str) -> impl std::future::Future<Output = anyhow::Result<String>> {
        return self.get(path).and_then(async |raw_file| {
            String::from_utf8(raw_file).context("While decoding utf8.")
        });
    }

    fn get_cp1252(&self, path: &str) -> impl std::future::Future<Output = anyhow::Result<String>> {
        return self.get(path).and_then(async |raw_file| {
            from_cp1252(Cursor::new(raw_file)).context("While decoding cp1252.")
        });
    }

    fn get_image(
        &self,
        path: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<image::DynamicImage>> {
        return self.get(path).and_then(async |raw_file| {
            image::load_from_memory(&raw_file).context("While loading image.")
        });
    }
}
