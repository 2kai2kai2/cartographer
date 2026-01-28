//! For values in `game/main_menu/setup/start/*.txt`, typically `10_countries.txt` in vanilla

use std::collections::HashMap;

use pdx_parser_core::TextDeserialize;
#[cfg(any())]
use pdx_parser_core::common_deserialize::SkipValue;

#[derive(TextDeserialize)]
pub struct Countries {
    /// Tag to setup country data
    pub countries: HashMap<String, Country>,
}

#[derive(TextDeserialize)]
pub struct Country {
    #[default(Vec::new())]
    pub own_control_core: Vec<String>,
    #[default(Vec::new())]
    pub own_control_integrated: Vec<String>,
    #[default(Vec::new())]
    pub own_control_colony: Vec<String>,
    #[default(Vec::new())]
    pub our_cores_conquered_by_others: Vec<String>,
    /// For pop-based countries
    #[default(Vec::new())]
    pub add_pops_from_locations: Vec<String>,
    #[multiple]
    pub include: Vec<String>,
    #[default(Vec::new())]
    pub discovered_areas: Vec<String>,
    #[default(Vec::new())]
    pub discovered_regions: Vec<String>,
    pub country_rank: Option<String>,
    pub starting_technology_level: Option<i32>,
    /// Seems to sometimes have multiple government fields, may have to merge them
    #[cfg(any())]
    pub government: SkipValue,
    pub capital: Option<String>,
    pub court_language: Option<String>,
    #[multiple]
    pub dynasty: Vec<String>,
    #[default(Vec::new())]
    pub accepted_cultures: Vec<String>,
    #[default(Vec::new())]
    pub tolerated_cultures: Vec<String>,
    #[cfg(any())]
    pub currency_data: Option<SkipValue>,
    #[cfg(any())]
    pub variables: Option<SkipValue>,
}
