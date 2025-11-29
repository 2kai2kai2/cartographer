use std::collections::HashMap;

use crate::eu5::{Countries, DiplomacyManager, RawMeta, SubunitManager, UnitManager};
use pdx_parser_macros::BinDeserialize;
use serde::{Deserialize, Serialize};

#[derive(BinDeserialize, Debug, Serialize, Deserialize)]
pub enum LocationRank {
    RuralSettlement,
    Town,
    City,
}

#[derive(BinDeserialize, Serialize, Deserialize)]
pub struct Location {
    #[bin_token("eu5")]
    pub owner: Option<u32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub controller: Option<u32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    previous_controller: Option<i32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub market: Option<u32>,
    // cores:
    #[cfg(any())]
    #[bin_token("eu5")]
    pub religion: Option<u32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub culture: Option<u32>,
    #[bin_token("eu5")]
    pub rank: Option<LocationRank>,
    // pub raw_material: &str
    // /// only if owned
    // pub max_raw_material_workers: Option<_>,
    // /// only if owned
    // pub development: Option<_>,
    // /// only if owned
    // pub control: Option<_>,
    /// Unowned locations and others where the tax is not set default to 0.0
    #[bin_token("eu5")]
    #[default(0.0)]
    pub tax: f64,
    /// Unowned locations and others where the possible_tax is not set default to 0.0
    #[bin_token("eu5")]
    #[default(0.0)]
    pub possible_tax: f64,
    // /// ID of the nation's province (multiple nations may have the same province, but each has a unique ID)
    // pub province: Option<_>, // only if owned
    // pub population:
}

#[derive(BinDeserialize, Serialize, Deserialize)]
pub struct Locations {
    #[bin_token("eu5")]
    pub locations: HashMap<i32, Location>,
}

#[derive(BinDeserialize, Serialize, Deserialize)]
pub struct PreviousPlayedItem {
    /// Country ID
    #[bin_token("eu5")]
    pub idtype: u32,
    /// Player name
    #[bin_token("eu5")]
    pub name: Box<str>,
}

#[derive(BinDeserialize, Serialize, Deserialize)]
#[no_brackets]
pub struct RawGamestate {
    #[bin_token("eu5")]
    pub metadata: RawMeta,
    #[bin_token("eu5")]
    pub countries: Countries,
    #[bin_token("eu5")]
    pub locations: Locations,
    #[bin_token("eu5")]
    pub previous_played: Vec<PreviousPlayedItem>,
    #[bin_token("eu5")]
    pub unit_manager: UnitManager,
    #[bin_token("eu5")]
    pub subunit_manager: SubunitManager,
    #[bin_token("eu5")]
    pub diplomacy_manager: DiplomacyManager,
}
