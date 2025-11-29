pub mod eu4;
pub mod eu5;
mod fetcher;
pub mod stellaris;
mod utils;

pub use eu4::{EU4ParserStepRawParsed, EU4ParserStepText};
pub use fetcher::*;
use pdx_parser_core::{eu4_save_parser, stellaris_save_parser};
use serde::{Deserialize, Serialize};
pub use stellaris::{StellarisParserStepRawParsed, StellarisParserStepText};

#[derive(Serialize, Deserialize)]
pub enum GameSaveType {
    // /// `.ck3` extension
    // CK3,
    /// `.eu4` extension
    EU4,
    /// `.eu5` extension
    EU5,
    /// `.sav` extension
    Stellaris,
    // /// `.v3` extension
    // Victoria3,
}
impl GameSaveType {
    /// String representation of the game (e.g. `eu4` or `stellaris`)
    pub fn id(&self) -> &'static str {
        return match self {
            &GameSaveType::EU4 => "eu4",
            &GameSaveType::EU5 => "eu5",
            &GameSaveType::Stellaris => "stellaris",
        };
    }
    pub fn determine_from_filename(filename: &str) -> Option<Self> {
        let filename_lower = filename.to_ascii_lowercase();
        if filename_lower.ends_with(".eu4") {
            return Some(GameSaveType::EU4);
        } else if filename_lower.ends_with(".eu5") {
            return Some(GameSaveType::EU5);
        } else if filename_lower.ends_with(".sav") {
            return Some(GameSaveType::Stellaris);
        } else {
            return None;
        }
    }
}

pub enum SomeSaveGame {
    EU4(eu4_save_parser::SaveGame),
    EU5(pdx_parser_core::eu5::RawGamestate),
    Stellaris(stellaris_save_parser::SaveGame),
}
impl SomeSaveGame {
    /// String representation of the game (e.g. `eu4` or `stellaris`)
    pub fn id(&self) -> &'static str {
        return match self {
            SomeSaveGame::EU4(_) => "eu4",
            SomeSaveGame::EU5(_) => "eu5",
            SomeSaveGame::Stellaris(_) => "stellaris",
        };
    }
}
