use serde::{Deserialize, Serialize};

#[cfg(any())]
use crate::common_deserialize::SkipValue;
use crate::{BinDeserialize, eu5_date::EU5Date};

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

/// It has brackets, but since the start bracket is a different token (need to figure that out),
/// so caller must remove the brackets before deserializing.
#[derive(BinDeserialize, Serialize, Deserialize)]
#[no_brackets]
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
