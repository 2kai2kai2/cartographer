use crate::{
    BinDeserialize, BinDeserializer, Context, bin_deserialize::BinError, bin_lexer::BinToken,
    common_deserialize::SkipValue,
};
#[cfg(any())]
use crate::{common_deserialize::SkipValue, eu5::EU5Date};
use pdx_parser_macros::eu5_token;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(BinDeserialize, Debug, Serialize, Deserialize)]
pub struct CountryRelation {
    #[cfg(any())]
    #[bin_token("eu5")]
    pub timed_biases: SkipValue,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub diplomat_return_date: Option<EU5Date>,
    /// Maybe a date?
    #[cfg(any())]
    #[bin_token("eu5")]
    pub last_war: Option<i32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub war_score: Option<i32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub currency: SkipValue,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub war: Option<u32>,
    /// Maybe a date?
    #[cfg(any())]
    #[bin_token("eu5")]
    pub last_spy_discovery: Option<i32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(false)]
    pub revolt: bool,
}

#[derive(BinDeserialize, Debug, Serialize, Deserialize)]
pub struct CountryDiplomacy {
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub diplomats: f64,
    /// Probably not present for most countries
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub num_relations_over_limit: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(Vec::new())]
    pub rivals: Vec<u32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(Vec::new())]
    pub enemies: Vec<u32>,
    /// Maybe a date?
    #[cfg(any())]
    #[bin_token("eu5")]
    pub last_war: Option<i32>,
    /// Maybe a date?
    #[cfg(any())]
    #[bin_token("eu5")]
    pub last_peace: Option<i32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(HashMap::new())]
    pub relations: HashMap<u32, CountryRelation>,
}

#[derive(BinDeserialize, Debug, Serialize, Deserialize)]
pub struct Dependency {
    /// Overlord
    #[bin_token("eu5")]
    pub first: u32,
    /// Subject
    #[bin_token("eu5")]
    pub second: u32,
    #[bin_token("eu5")]
    pub subject_type: Box<str>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub start_date: Option<EU5Date>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub seed: Option<u32>,
}

/// Contains both `u32` keys which are a map of country ids, as well as token-based keys
#[derive(Debug, Serialize, Deserialize)]
pub struct DiplomacyManager {
    /// All u32 keys
    #[cfg(any())]
    pub countries: HashMap<u32, CountryDiplomacy>,
    /// Subject id -> dependency
    ///
    /// Derived from all `dependency=` entries
    pub overlords: HashMap<u32, Dependency>,
}
impl<'de> BinDeserialize<'de> for DiplomacyManager {
    fn take(
        mut stream: BinDeserializer<'de>,
    ) -> ::std::result::Result<(Self, BinDeserializer<'de>), BinError> {
        stream.parse_token(BinToken::ID_OPEN_BRACKET)?;
        #[cfg(any())]
        let mut countries = HashMap::new();
        let mut overlords = HashMap::new();

        loop {
            match stream.peek_token().ok_or(BinError::EOF)? {
                BinToken::ID_EQUAL => {
                    return Err(BinError::UnexpectedToken(BinToken::ID_EQUAL));
                }
                BinToken::ID_CLOSE_BRACKET => {
                    stream.eat_token();
                    break;
                }
                #[cfg(any())]
                BinToken::ID_U32 => {
                    // Country ID
                    let country = stream.parse()?;
                    let Ok(_) = stream.parse_token(BinToken::ID_EQUAL) else {
                        continue; // shouldn't happen since DiplomacyManager should only have KVs, but just in case
                    };
                    let country_diplomacy = stream.parse()?;
                    countries.insert(country, country_diplomacy);
                }
                eu5_token!("dependency") => {
                    stream.eat_token();
                    let Ok(_) = stream.parse_token(BinToken::ID_EQUAL) else {
                        continue; // shouldn't happen since DiplomacyManager should only have KVs, but just in case
                    };
                    let value: Dependency = stream.parse().context("While parsing dependency")?;
                    overlords.insert(value.second, value);
                }
                _ => {
                    // ignore
                    let SkipValue = stream.parse()?;
                    if let Ok(()) = stream.parse_token(BinToken::ID_EQUAL) {
                        let SkipValue = stream.parse()?;
                    };
                }
            }
        }

        return Ok((
            DiplomacyManager {
                #[cfg(any())]
                countries,
                overlords,
            },
            stream,
        ));
    }
}
