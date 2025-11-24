pub mod assets;
pub mod eu5_map;
pub mod stats_image;

use anyhow::anyhow;
use pdx_parser_core::{
    eu5_gamestate::RawGamestate,
        modern_header::{ModernHeader, SaveFormat},
    BinDeserializer,
};
use std::io::Read;
use zip::ZipArchive;

pub use eu5_map::*;
pub use stats_image::*;

pub struct EU5ParserStepGamestate(Box<[u8]>);
impl EU5ParserStepGamestate {
    pub fn decompress_from(file_buf: &[u8]) -> anyhow::Result<EU5ParserStepGamestate> {
        let file = ModernHeader::take(file_buf)
            .ok_or(anyhow!("Failed to parse EU5 save file header format."))?;
        if file.save_format != SaveFormat::UnifiedCompressedBinary {
            return Err(anyhow!(
                "We currently only support unified compressed binary format save files."
            ));
        }

        let mut archive = ZipArchive::new(std::io::Cursor::new(file.gamestate))?;
        let mut gamestate = archive.by_name("gamestate")?;
        let mut buf = Vec::new();
        gamestate.read_to_end(&mut buf)?;
        return Ok(EU5ParserStepGamestate(buf.into()));
    }

    pub fn parse(self) -> anyhow::Result<RawGamestate> {
        let mut deserializer = BinDeserializer::from_bytes(&self.0);
        let gamestate = deserializer
            .parse()
            .map_err(|err| anyhow!("Failed to parse save file (at step 1): {err}"))?;
        return Ok(gamestate);
    }
}
