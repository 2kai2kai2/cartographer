pub mod eu4;
mod fetcher;
pub mod stellaris;
use anyhow::anyhow;
pub use fetcher::*;
use pdx_parser_core::{eu4_save_parser, raw_parser::RawPDXObject, stellaris_save_parser};
use serde::{Deserialize, Serialize};

use crate::{eu4::decompress_eu4txt, stellaris::decompress_stellaris};

#[derive(Serialize, Deserialize)]
pub enum GameSaveType {
    // /// `.ck3` extension
    // CK3,
    /// `.eu4` extension
    EU4,
    // /// TBD extension
    // EU5,
    /// `.sav` extension
    Stellaris,
    // /// `.v3` extension
    // Victoria3,
}

pub enum SomeSaveGame {
    EU4(eu4_save_parser::SaveGame),
    Stellaris(stellaris_save_parser::SaveGame),
}

pub fn parse_save_file(array: &[u8], filename: &str) -> anyhow::Result<SomeSaveGame> {
    // Need to figure out what game this file is for
    let filename_lower = filename.to_ascii_lowercase();
    if filename_lower.ends_with(".eu4") {
        let save = if array.starts_with("EU4txt".as_bytes()) {
            // log!("Detected uncompressed eu4 save file");
            from_cp1252(array)?
        } else if array.starts_with("PK\x03\x04".as_bytes()) {
            // log!("Detected compressed file");
            decompress_eu4txt(array)?
        } else {
            return Err(
                anyhow!("Initial check shows file does not seem to be a valid eu4 format",).into(),
            );
        };
        let (_, save) = RawPDXObject::parse_object_inner(&save)
            .ok_or(anyhow!("Failed to parse save file (at step 1)"))?;
        let save = eu4_save_parser::SaveGame::new_parser(&save)
            .ok_or(anyhow!("Failed to parse save file (at step 2)"))?;
        return Ok(SomeSaveGame::EU4(save));
    } else if filename_lower.ends_with(".sav") {
        // seems like all stellaris saves are compressed
        if !array.starts_with("PK\x03\x04".as_bytes()) {
            return Err(anyhow!(
                "Stellaris save was not a proper zip-compressed file."
            ));
        }
        let save = decompress_stellaris(array)?;
        let (_, save) = RawPDXObject::parse_object_inner(&save)
            .ok_or(anyhow!("Failed to parse save file (at step 1)"))?;
        let save = stellaris_save_parser::SaveGame::new_parser(&save)?;

        return Ok(SomeSaveGame::Stellaris(save));
    } else {
        // TODO: try to figure it out from context.
        return Err(anyhow!(
            "Could not determine the save format. Did you change the file extension?",
        ));
    }
}
