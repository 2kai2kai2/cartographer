pub mod assets;
pub mod eu5_map;
pub mod stats_image;

use anyhow::{Context, anyhow};
pub use eu5_map::*;
use pdx_parser_core::{
    BinDeserializer, StringsResolver,
    eu5::RawGamestate,
    modern_header::{ModernHeader, SaveFormat},
};
pub use stats_image::*;
use std::io::Read;
use zip::ZipArchive;

pub struct EU5ParserStepGamestate {
    string_lookup: Box<[u8]>,
    gamestate: Box<[u8]>,
}
impl EU5ParserStepGamestate {
    pub fn decompress_from(file_buf: &[u8]) -> anyhow::Result<EU5ParserStepGamestate> {
        let file =
            ModernHeader::take(file_buf).context("Failed to parse EU5 save file header format.")?;
        if file.save_format != SaveFormat::UnifiedCompressedBinary {
            return Err(anyhow!(
                "We currently only support unified compressed binary format save files."
            ));
        }

        let mut archive = ZipArchive::new(std::io::Cursor::new(file.gamestate))?;
        let mut string_lookup_zip = archive.by_name("string_lookup")?;
        let mut string_lookup = Vec::new();
        string_lookup_zip.read_to_end(&mut string_lookup)?;
        drop(string_lookup_zip);

        let mut gamestate_zip = archive.by_name("gamestate")?;
        let mut gamestate = Vec::new();
        gamestate_zip.read_to_end(&mut gamestate)?;
        drop(gamestate_zip);
        drop(archive);

        return Ok(EU5ParserStepGamestate {
            string_lookup: string_lookup.into(),
            gamestate: gamestate.into(),
        });
    }

    pub fn parse(self) -> anyhow::Result<RawGamestate> {
        let strings = StringsResolver::from_raw(self.string_lookup)
            .context("While parsing `string_lookup` for eu5 save")?;
        let mut deserializer = BinDeserializer::from_bytes(&self.gamestate, &strings);
        let gamestate = deserializer
            .parse()
            .map_err(|err| anyhow!("Failed to parse save file (at step 1): {err}"))?;
        return Ok(gamestate);
    }
}
