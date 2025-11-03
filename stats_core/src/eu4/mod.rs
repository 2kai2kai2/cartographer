pub mod assets;
mod eu4_map;
mod stats_image;
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
