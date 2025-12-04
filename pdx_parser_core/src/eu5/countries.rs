//! Everything in the `countries` section of the save file

#[cfg(any())]
use crate::common_deserialize::SkipValue;
use crate::{
    BinDeserialize, BinDeserializer, bin_deserialize::BinError, bin_lexer::BinToken,
    common_deserialize::Rgb, eu5::RawGamestate,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(BinDeserialize, Serialize, Deserialize)]
pub struct CountryEconomy {
    /// estate to proportion
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(HashMap::new())]
    pub tax_rates: HashMap<String, f64>,
    /// maintenance item to proportion
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(HashMap::new())]
    pub maintenances: HashMap<String, f64>,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub total_debt: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub coin_minting: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub loan_capacity: f64,
    #[bin_token("eu5")]
    #[default(0.0)]
    pub expense: f64,
    #[bin_token("eu5")]
    #[default(0.0)]
    pub income: f64,
    /// Up to 12 months income? values
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(Vec::new())]
    pub monthly_gold: Vec<f64>,
    /// Up to 12 months balance? values
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(Vec::new())]
    pub recent_balance: Vec<f64>,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(HashMap::new())]
    pub counters: HashMap<String, SkipValue>,
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
    #[cfg(any())]
    #[bin_token("eu5")]
    pub flag: SkipValue,
    #[bin_token("eu5")]
    pub definition: Option<Box<str>>,
    #[bin_token("eu5", "type")]
    pub country_basis_type: CountryBasisType,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub historical: SkipValue,
    #[bin_token("eu5")]
    pub country_type: CountryType,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub score: SkipValue,
    #[bin_token("eu5")]
    pub currency_data: CountryCurrencyData,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0)]
    pub artists: i32,
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
    pub economy: CountryEconomy,
    /// Usually present
    #[bin_token("eu5")]
    pub color: Option<Rgb>,
    #[bin_token("eu5")]
    pub color2: Option<Rgb>,
    #[bin_token("eu5")]
    pub color3: Option<Rgb>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub unit_color0: Option<Rgb>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub unit_color1: Option<Rgb>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub unit_color2: Option<Rgb>,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(false)]
    pub great_power: bool,
    #[bin_token("eu5")]
    pub great_power_rank: Option<i32>,
    #[bin_token("eu5")]
    pub capital: Option<i32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub original_capital: Option<i32>,
    /// total population last month, in thousands
    #[bin_token("eu5")]
    #[default(f64::NAN)]
    pub last_months_population: f64,
    #[bin_token("eu5")]
    #[default(Box::new([]))]
    pub units: Box<[u32]>,
    #[bin_token("eu5")]
    #[default(Box::new([]))]
    pub owned_subunits: Box<[u32]>,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(Vec::new())]
    pub historical_population: Vec<f64>,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(Vec::new())]
    pub historical_tax_base: Vec<f64>,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub naval_range: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub religious_school: Option<Box<str>>,
    #[cfg(any())]
    #[bin_token("eu5", "dyn")]
    #[default(false)]
    pub dynamic: bool,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(false)]
    pub revolt: bool,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub rebel: Option<Box<str>>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub formed_from: Option<Box<str>>,
}
impl RawCountry {
    /// Returns (regular_army, regular_navy, levy_army, levy_navy) where
    /// - Levies only include raised levies
    /// - Army is in total number of troops
    /// - Navy is in total number of ships
    ///
    pub fn military_size(&self, gamestate: &RawGamestate) -> anyhow::Result<(u64, u64, u64, u64)> {
        let mut regular_army = 0;
        let mut regular_navy = 0;
        let mut levy_army = 0;
        let mut levy_navy = 0;

        for subunit_id in self.owned_subunits.iter().cloned() {
            let subunit = gamestate
                .subunit_manager
                .database
                .get(&subunit_id)
                .ok_or(anyhow::anyhow!("Failed to find subunit {}", subunit_id))?;
            let subunit = subunit.as_unit().ok_or(anyhow::anyhow!(
                "Tried to get subunit {} but it was `none`",
                subunit_id
            ))?;
            let unit =
                gamestate
                    .unit_manager
                    .database
                    .get(&subunit.unit)
                    .ok_or(anyhow::anyhow!(
                        "Failed to find unit {} for subunit {}",
                        subunit.unit,
                        subunit_id
                    ))?;
            let unit = unit.as_unit().ok_or(anyhow::anyhow!(
                "Tried to get unit {} for subunit {} but it was `none`",
                subunit.unit,
                subunit_id
            ))?;

            match (unit.is_army, &subunit.levies) {
                (true, None) => {
                    regular_army += (subunit.strength * 1000.0) as u64;
                }
                (false, None) => {
                    regular_navy += 1;
                }
                (true, Some(_)) => {
                    levy_army += (subunit.strength * 1000.0) as u64;
                }
                (false, Some(_)) => {
                    levy_navy += 1;
                }
            }
        }
        return Ok((regular_army, regular_navy, levy_army, levy_navy));
    }
}

#[derive(BinDeserialize, Serialize, Deserialize)]
pub struct Countries {
    #[bin_token("eu5")]
    pub tags: HashMap<u32, Box<str>>,
    #[bin_token("eu5")]
    pub database: HashMap<u32, RawCountriesEntry>,
}
