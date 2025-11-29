//! Everything in the `subunit_manager` section of the save file

use crate::{
    BinDeserialize, BinDeserializer, bin_deserialize::BinError, bin_lexer::BinToken,
    common_deserialize::SkipValue,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(BinDeserialize, Debug, Serialize, Deserialize)]
pub struct RawSubunit {
    #[cfg(any())]
    #[bin_token("eu5")]
    pub subunit_name_2: String,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub owner: u32,
    /// May differ from owner if it's a prisoner
    #[cfg(any())]
    #[bin_token("eu5")]
    pub controller: u32,
    /// Seems to be the flank it's in? idk
    #[cfg(any())]
    #[bin_token("eu5", "box")]
    pub subunit_box: Option<Box<str>>,
    /// The unit it is a part of
    #[bin_token("eu5")]
    pub unit: u32,
    /// Probably its home location
    #[cfg(any())]
    #[bin_token("eu5")]
    pub home: i32,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub culture: u32,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub religion: u32,
    #[cfg(any())]
    #[bin_token("eu5", "type")]
    pub subunit_type: Box<str>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub morale: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub experience: f64,
    /// In thousands for armies
    #[bin_token("eu5")]
    #[default(1.0)]
    pub strength: f64,
    /// In thousands for armies
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(1.0)]
    pub max_strength: f64,
    /// Attrition losses over the last few months
    #[cfg(any())]
    #[bin_token("eu5")]
    pub attrition_losses_per_month: SkipValue,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0)]
    pub last_month_for_attrition: i32,

    /// Seems to be data if it is in battle
    #[cfg(any())]
    #[bin_token("eu5")]
    pub target: Option<u32>,
    /// Seems to be about whether it is engaged in battle
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(false)]
    pub engaged: bool,
    /// Data about goods needs and how much is missing
    #[cfg(any())]
    #[bin_token("eu5")]
    pub missing: SkipValue,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub number: u32,
    /// Only exists on levy subunits, unclear what the data represents
    /// For now, we just need to know it's a levy so just skip the internal data
    #[bin_token("eu5")]
    pub levies: Option<SkipValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RawSubunitEntry {
    None,
    Unit(RawSubunit),
}
impl RawSubunitEntry {
    pub fn to_unit(self) -> Option<RawSubunit> {
        match self {
            RawSubunitEntry::None => None,
            RawSubunitEntry::Unit(subunit) => Some(subunit),
        }
    }
    pub fn as_unit(&self) -> Option<&RawSubunit> {
        match self {
            RawSubunitEntry::None => None,
            RawSubunitEntry::Unit(subunit) => Some(subunit),
        }
    }
}
impl<'de> BinDeserialize<'de> for RawSubunitEntry {
    fn take(
        mut stream: BinDeserializer<'de>,
    ) -> ::std::result::Result<(Self, BinDeserializer<'de>), BinError> {
        let peek = stream.peek_token().ok_or(BinError::EOF)?;
        match peek {
            BinToken::ID_OPEN_BRACKET => {
                let (value, rest) = RawSubunit::take(stream)?;
                return Ok((RawSubunitEntry::Unit(value), rest));
            }
            pdx_parser_macros::eu5_token!("none") => {
                stream.eat_token();
                return Ok((RawSubunitEntry::None, stream));
            }
            _ => {
                return Err(BinError::UnexpectedToken(peek));
            }
        }
    }
}

#[derive(BinDeserialize, Debug, Serialize, Deserialize)]
pub struct SubunitManager {
    #[bin_token("eu5")]
    pub database: HashMap<u32, RawSubunitEntry>,
}
