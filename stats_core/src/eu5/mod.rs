pub mod assets;
pub mod eu5_map;
pub mod stats_image;

use anyhow::anyhow;
use pdx_parser_core::{
    eu5_gamestate::RawGamestate,
    eu5_meta::RawMeta,
    modern_header::{ModernHeader, SaveFormat},
    BinDeserializer,
};
use std::io::Read;
use zip::ZipArchive;

pub use eu5_map::*;
pub use stats_image::*;

pub struct EU5ParserStepGamestate {
    meta: Box<[u8]>,
    gamestate: Box<[u8]>,
}
impl EU5ParserStepGamestate {
    pub fn decompress_from(file_buf: &[u8]) -> anyhow::Result<EU5ParserStepGamestate> {
        let (body, header) = ModernHeader::take(file_buf)
            .ok_or(anyhow!("Failed to parse EU5 save file header format."))?;
        if header.save_format != SaveFormat::UnifiedCompressedBinary {
            return Err(anyhow!(
                "We currently only support unified compressed binary format save files."
            ));
        }

        let mut archive = ZipArchive::new(std::io::Cursor::new(body))?;
        let mut gamestate = archive.by_name("gamestate")?;
        let mut buf = Vec::new();
        gamestate.read_to_end(&mut buf)?;
        return Ok(EU5ParserStepGamestate {
            meta: header.meta.into(),
            gamestate: buf.into(),
        });
    }

    pub fn parse(self) -> anyhow::Result<(RawMeta, RawGamestate)> {
        let stripped_meta = self
            .meta
            .get(14..)
            .ok_or(anyhow!("Failed to strip meta brackets"))?;
        let mut deserializer = BinDeserializer::from_bytes(stripped_meta);
        let meta = deserializer
            .parse()
            .map_err(|err| anyhow!("Failed to parse save file (meta at step 1): {err}"))?;

        let mut deserializer = BinDeserializer::from_bytes(&self.gamestate);
        let gamestate = deserializer
            .parse()
            .map_err(|err| anyhow!("Failed to parse save file (gamestateat step 1): {err}"))?;
        return Ok((meta, gamestate));
    }
}
