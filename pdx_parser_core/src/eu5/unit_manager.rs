//! Everything in the `unit_manager` section of the save file

#[cfg(any())]
use crate::common_deserialize::SkipValue;
use crate::{BinDeserialize, BinDeserializer, bin_deserialize::BinError, bin_lexer::BinToken};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(BinDeserialize, Debug, Serialize, Deserialize)]
pub struct RawUnit {
    /// If true, is land army, if false, is navy
    #[bin_token("eu5")]
    #[default(false)]
    pub is_army: bool,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub unit_name: SkipValue,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub movement_progress: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(1.0)]
    pub attrition_weight: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub country: u32,
    // some food fields here
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(1.0)]
    pub frontage: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub speed: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub leader: Option<u32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub arrived_here_date: i32,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub location: i32,
    /// Previous location
    #[cfg(any())]
    #[bin_token("eu5")]
    pub previous: Option<i32>,
    /// Previous non-zone-of-control location?
    #[cfg(any())]
    #[bin_token("eu5")]
    pub non_zoc: Option<f64>,
    /// Last port it was at if it is a navy
    #[cfg(any())]
    #[bin_token("eu5")]
    pub last_port: Option<i32>,
    /// Blockade capacity if it is a navy
    #[cfg(any())]
    #[bin_token("eu5")]
    pub blockade_capacity: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RawUnitEntry {
    None,
    Unit(RawUnit),
}
impl RawUnitEntry {
    pub fn to_unit(self) -> Option<RawUnit> {
        match self {
            RawUnitEntry::None => None,
            RawUnitEntry::Unit(unit) => Some(unit),
        }
    }
    pub fn as_unit(&self) -> Option<&RawUnit> {
        match self {
            RawUnitEntry::None => None,
            RawUnitEntry::Unit(unit) => Some(unit),
        }
    }
}
impl<'de> BinDeserialize<'de> for RawUnitEntry {
    fn take(
        mut stream: BinDeserializer<'de>,
    ) -> ::std::result::Result<(Self, BinDeserializer<'de>), BinError> {
        let peek = stream.peek_token().ok_or(BinError::EOF)?;
        match peek {
            BinToken::ID_OPEN_BRACKET => {
                let (value, rest) = RawUnit::take(stream)?;
                return Ok((RawUnitEntry::Unit(value), rest));
            }
            pdx_parser_macros::eu5_token!("none") => {
                stream.eat_token();
                return Ok((RawUnitEntry::None, stream));
            }
            _ => {
                return Err(BinError::UnexpectedToken(peek));
            }
        }
    }
}

#[derive(BinDeserialize, Debug, Serialize, Deserialize)]
pub struct UnitManager {
    #[bin_token("eu5")]
    pub database: HashMap<u32, RawUnitEntry>,
}
