pub mod assets;
pub mod flags;
mod stats_image;
mod stellaris_map;
use anyhow::anyhow;
use pdx_parser_core::raw_parser::RawPDXObject;
use pdx_parser_core::stellaris_save_parser::SaveGame;
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

pub struct StellarisParserStepText(String);
impl StellarisParserStepText {
    pub fn decode_from(file_buf: &[u8]) -> anyhow::Result<StellarisParserStepText> {
        if !file_buf.starts_with(b"PK\x03\x04") {
            return Err(anyhow!(
                "Stellaris save was not a proper zip-compressed file."
            ));
        }
        let text = decompress_stellaris(file_buf)?;
        return Ok(StellarisParserStepText(text));
    }
    pub fn parse<'a>(&'a self) -> anyhow::Result<StellarisParserStepRawParsed<'a>> {
        let (_, raw_save) = RawPDXObject::parse_object_inner(&self.0)
            .ok_or(anyhow!("Failed to parse Stellaris save file (at step 1)"))?;
        return Ok(StellarisParserStepRawParsed(raw_save));
    }
}
pub struct StellarisParserStepRawParsed<'a>(RawPDXObject<'a>);
impl<'a> StellarisParserStepRawParsed<'a> {
    pub fn parse(self) -> anyhow::Result<SaveGame> {
        return SaveGame::new_parser(&self.0)
            .map_err(|err| anyhow!("Failed to parse save file (at step 2): {err}"));
    }
}
