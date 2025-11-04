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
impl SomeSaveGame {
    /// String representation of the game (e.g. `eu4` or `stellaris`)
    pub fn id(&self) -> &'static str {
        return match self {
            SomeSaveGame::EU4(_) => "eu4",
            SomeSaveGame::Stellaris(_) => "stellaris",
        };
    }
}

pub enum PreprocessedSaveGame {
    EU4(String),
    Stellaris(String),
}
impl PreprocessedSaveGame {
    pub fn preprocess_eu4(file_buf: &[u8]) -> anyhow::Result<String> {
        if file_buf.starts_with("EU4txt".as_bytes()) {
            return Ok(from_cp1252(file_buf)?);
        } else if file_buf.starts_with("PK\x03\x04".as_bytes()) {
            return decompress_eu4txt(file_buf);
        } else {
            return Err(
                anyhow!("Initial check shows file does not seem to be a valid eu4 format",).into(),
            );
        };
    }
    pub fn preprocess_stellaris(file_buf: &[u8]) -> anyhow::Result<String> {
        if !file_buf.starts_with("PK\x03\x04".as_bytes()) {
            return Err(anyhow!(
                "Stellaris save was not a proper zip-compressed file."
            ));
        }
        return decompress_stellaris(file_buf);
    }
    /// Determines what game the file is for and normalizes into the decompressed, utf8-based text format.
    pub fn preprocess(file_buf: &[u8], filename: &str) -> anyhow::Result<PreprocessedSaveGame> {
        let filename_lower = filename.to_ascii_lowercase();
        if filename_lower.ends_with(".eu4") {
            let text_save = PreprocessedSaveGame::preprocess_eu4(file_buf)?;
            return Ok(PreprocessedSaveGame::EU4(text_save));
        } else if filename_lower.ends_with(".sav") {
            let text_save = PreprocessedSaveGame::preprocess_stellaris(file_buf)?;
            return Ok(PreprocessedSaveGame::Stellaris(text_save));
        } else {
            // TODO: try to figure it out from context.
            return Err(anyhow!(
                "Could not determine the save format. Did you change the file extension?",
            ));
        }
    }
    /// Converts the string in the `PreprocessedSaveGame` into a raw parsed save.
    pub fn to_raw_parsed<'a>(&'a self) -> anyhow::Result<RawPDXObject<'a>> {
        match self {
            PreprocessedSaveGame::EU4(save) => {
                let (_, save) = RawPDXObject::parse_object_inner(&save)
                    .ok_or(anyhow!("Failed to parse save file (at step 1)"))?;
                return Ok(save);
                // let save = eu4_save_parser::SaveGame::new_parser(&save)
                //     .ok_or(anyhow!("Failed to parse save file (at step 2)"))?;
                // return Ok(SomeSaveGame::EU4(save));
            }
            PreprocessedSaveGame::Stellaris(save) => {
                let (_, save) = RawPDXObject::parse_object_inner(&save)
                    .ok_or(anyhow!("Failed to parse save file (at step 1)"))?;
                return Ok(save);
                // let save = stellaris_save_parser::SaveGame::new_parser(&save)?;
                // return Ok(SomeSaveGame::Stellaris(save));
            }
        }
    }
}
