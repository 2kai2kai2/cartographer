pub mod assets;
pub mod flags;
mod stats_image;
mod stellaris_map;
use std::io::Cursor;
use std::io::Read;

pub use stats_image::*;
pub use stellaris_map::*;

pub const JURA_MEDIUM_TTF: &[u8] =
    include_bytes!("../../../cartographer_web/resources/stellaris/Jura-Medium.ttf");
pub const STELLARIS_MAP_IMAGE_SIZE: u32 = 2048;

pub fn decompress_stellaris(array: &[u8]) -> anyhow::Result<String> {
    let mut cursor = Cursor::new(array);
    let mut unzipper = zip::read::ZipArchive::new(&mut cursor)?;

    // let mut unzipped_meta = unzipper.by_name("meta")?;
    // let mut meta = String::new();
    // unzipped_meta.read_to_string(&mut meta)?;

    let mut unzipped_gamestate = unzipper.by_name("gamestate")?;
    let mut gamestate = String::new();
    unzipped_gamestate.read_to_string(&mut gamestate)?;
    return Ok(gamestate);
}
