use std::collections::HashMap;

use crate::{
    bin_deserialize::{BinDeserialize, BinDeserializer, BinError},
    bin_lexer::BinToken,
    common_deserialize::Rgb,
    eu5_meta::RawMeta,
};
use pdx_parser_macros::BinDeserialize;
use serde::{Deserialize, Serialize};

#[derive(BinDeserialize, Debug, Serialize, Deserialize)]
pub enum CountryBasisType {
    Location,
    Building,
    Army,
    Navy,
    Pop,
}

#[derive(Serialize, Deserialize)]
pub enum CountryType {
    Pirates,
    Mercenaries,
    Real,
    Unknown(Box<str>),
}
impl<'de> BinDeserialize<'de> for CountryType {
    fn take(mut stream: BinDeserializer<'de>) -> Result<(Self, BinDeserializer<'de>), BinError> {
        let text: &'de str = stream.parse()?;
        let out = match text {
            "pirates" => CountryType::Pirates,
            "mercenaries" => CountryType::Mercenaries,
            "real" => CountryType::Real,
            other => CountryType::Unknown(other.into()),
        };
        return Ok((out, stream));
    }
}

#[derive(BinDeserialize, Serialize, Deserialize)]
pub struct CountryCurrencyData {
    #[bin_token("eu5")]
    #[default(0.0)]
    pub gold: f64,
    #[bin_token("eu5")]
    #[default(0.0)]
    pub manpower: f64,
    #[bin_token("eu5")]
    #[default(0.0)]
    pub sailors: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub stability: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub inflation: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub prestige: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub army_tradition: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub navy_tradition: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub government_power: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(f64::NAN)]
    pub religious_influence: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub purity: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub righteousness: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub karma: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub self_control: f64,
}

#[derive(Serialize, Deserialize)]
pub enum RawCountriesEntry {
    None,
    Country(RawCountry),
}
impl RawCountriesEntry {
    pub fn to_country(self) -> Option<RawCountry> {
        match self {
            RawCountriesEntry::None => None,
            RawCountriesEntry::Country(country) => Some(country),
        }
    }
    pub fn as_country(&self) -> Option<&RawCountry> {
        match self {
            RawCountriesEntry::None => None,
            RawCountriesEntry::Country(country) => Some(country),
        }
    }
}
impl<'de> BinDeserialize<'de> for RawCountriesEntry {
    fn take(
        mut stream: BinDeserializer<'de>,
    ) -> ::std::result::Result<(Self, BinDeserializer<'de>), BinError> {
        let peek = stream.peek_token().ok_or(BinError::EOF)?;
        match peek {
            BinToken::ID_OPEN_BRACKET => {
                let (value, rest) = RawCountry::take(stream)?;
                return Ok((RawCountriesEntry::Country(value), rest));
            }
            pdx_parser_macros::eu5_token!("none") => {
                stream.eat_token();
                return Ok((RawCountriesEntry::None, stream));
            }
            _ => {
                return Err(BinError::UnexpectedToken(peek));
            }
        }
    }
}

#[derive(BinDeserialize, Serialize, Deserialize)]
pub struct RawCountry {
    #[bin_token("eu5")]
    pub definition: Option<Box<str>>,
    #[bin_token("eu5", "type")]
    pub country_basis_type: CountryBasisType,
    #[bin_token("eu5")]
    pub country_type: CountryType,
    #[bin_token("eu5")]
    pub currency_data: CountryCurrencyData,
    #[bin_token("eu5")]
    #[default(0.0)]
    pub estimated_monthly_income: f64,
    /// Tax base after control
    #[default(0.0)]
    #[bin_token("eu5")]
    pub current_tax_base: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub monthly_manpower: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub monthly_sailors: f64,
    #[bin_token("eu5")]
    #[default(0.0)]
    pub max_manpower: f64,
    #[bin_token("eu5")]
    #[default(0.0)]
    pub max_sailors: f64,
    #[bin_token("eu5")]
    pub color: Option<Rgb>,
    #[bin_token("eu5")]
    pub great_power_rank: Option<i32>,
    #[bin_token("eu5")]
    pub capital: Option<i32>,
    /// total population last month, in thousands
    #[bin_token("eu5")]
    #[default(f64::NAN)]
    pub last_months_population: f64,
}

#[derive(BinDeserialize, Serialize, Deserialize)]
pub struct Countries {
    #[bin_token("eu5")]
    pub tags: HashMap<u32, Box<str>>,
    #[bin_token("eu5")]
    pub database: HashMap<u32, RawCountriesEntry>,
}

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
}
