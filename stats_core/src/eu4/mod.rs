pub mod assets;
mod eu4_map;
mod stats_image;
use anyhow::anyhow;
use pdx_parser_core::{eu4_save_parser::SaveGame, raw_parser::RawPDXObject};
use std::io::Cursor;

pub use eu4_map::*;
pub use stats_image::*;

use crate::fetcher::from_cp1252;

pub const GARAMOND_TTF: &[u8] = include_bytes!("../../../cartographer_web/resources/eu4/GARA.TTF");

pub fn decompress_eu4txt(array: &[u8]) -> anyhow::Result<String> {
    let mut cursor = Cursor::new(array);
    let mut unzipper = zip::read::ZipArchive::new(&mut cursor)?;

    let unzipped_meta = unzipper.by_name("meta")?;
    let meta = from_cp1252(unzipped_meta)?;

    let unzipped_gamestate = unzipper.by_name("gamestate")?;
    let gamestate = from_cp1252(unzipped_gamestate)?;
    return Ok(meta + "\n" + &gamestate);
}

pub struct EU4ParserStepText(String);
impl EU4ParserStepText {
    pub fn decode_from(file_buf: &[u8]) -> anyhow::Result<EU4ParserStepText> {
        if !file_buf.starts_with(b"PK\x03\x04") {
            return Err(anyhow!("EU4 save was not a proper zip-compressed file."));
        }
        let text = decompress_eu4txt(file_buf)?;
        return Ok(EU4ParserStepText(text));
    }
    pub fn parse<'a>(&'a self) -> anyhow::Result<EU4ParserStepRawParsed<'a>> {
        let (_, raw_save) = RawPDXObject::parse_object_inner(&self.0)
            .ok_or(anyhow!("Failed to parse EU4 save file (at step 1)"))?;
        return Ok(EU4ParserStepRawParsed(raw_save));
    }
}
pub struct EU4ParserStepRawParsed<'a>(RawPDXObject<'a>);
impl<'a> EU4ParserStepRawParsed<'a> {
    pub fn parse(self) -> anyhow::Result<SaveGame> {
        return SaveGame::new_parser(&self.0)
            .ok_or(anyhow!("Failed to parse save file (at step 2)"));
    }
}
