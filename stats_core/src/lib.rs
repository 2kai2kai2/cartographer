pub mod eu4;
mod fetcher;
pub mod stellaris;

pub use eu4::{EU4ParserStepRawParsed, EU4ParserStepText};
pub use fetcher::*;
use pdx_parser_core::{eu4_save_parser, eu5_deserialize, stellaris_save_parser};
use serde::{Deserialize, Serialize};
pub use stellaris::{StellarisParserStepRawParsed, StellarisParserStepText};

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
impl GameSaveType {
    /// String representation of the game (e.g. `eu4` or `stellaris`)
    pub fn id(&self) -> &'static str {
        return match self {
            &GameSaveType::EU4 => "eu4",
            &GameSaveType::Stellaris => "stellaris",
        };
    }
    pub fn determine_from_filename(filename: &str) -> Option<Self> {
        let filename_lower = filename.to_ascii_lowercase();
        if filename_lower.ends_with(".eu4") {
            return Some(GameSaveType::EU4);
        } else if filename_lower.ends_with(".sav") {
            return Some(GameSaveType::Stellaris);
        } else {
            return None;
        }
    }
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
