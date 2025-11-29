#[cfg(any())]
use crate::common_deserialize::SkipValue;
use crate::{BinDeserialize, eu5::EU5Date};
use serde::{Deserialize, Serialize};

#[derive(BinDeserialize, Serialize, Deserialize)]
pub struct RawCompatibility {
    #[bin_token("eu5")]
    pub version: i32,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub locations_hash: u64,
    /// Locations, indexed by their id in the save file
    #[bin_token("eu5")]
    pub locations: Vec<String>,
}

#[derive(BinDeserialize, Serialize, Deserialize)]
pub struct RawMeta {
    #[bin_token("eu5")]
    pub date: EU5Date,
    #[bin_token("eu5")]
    pub playthrough_id: String,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub playthrough_name: String,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub save_label: String,
    #[bin_token("eu5")]
    pub version: String,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub incompatible: bool,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub flag: String,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub player_country_name: String,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub code_version_info: SkipValue,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub enabled_dlcs: Vec<String>,
    #[bin_token("eu5")]
    pub compatibility: RawCompatibility,
}
