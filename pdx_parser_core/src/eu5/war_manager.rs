use crate::{
    BinDeserialize, BinDeserializer, bin_deserialize::BinError, bin_lexer::BinToken,
    common_deserialize::SkipValue, eu5::EU5Date,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct WarManager {
    #[bin_token("eu5")]
    pub names: SkipValue,
    #[bin_token("eu5")]
    pub database: HashMap<u32, RawWarsEntry>,
}

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct RawWar {
    #[bin_token("eu5")]
    pub all: Vec<WarParticipant>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub original_attacker: u32,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub original_defenders: Vec<u32>,
    #[bin_token("eu5")]
    pub war_name: SkipValue,
    #[bin_token("eu5")]
    pub start_date: EU5Date,
    #[bin_token("eu5")]
    pub end_date: Option<EU5Date>,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(false)]
    pub previous: bool,
    /// Seems to be location -> occupier
    #[cfg(any())]
    #[bin_token("eu5")]
    pub locations: HashMap<i32, u32>,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(false)]
    pub has_civil_war: bool,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(false)]
    pub revolt: bool,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub defender_score: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    #[default(0.0)]
    pub attacker_score: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub stalled_years: i32,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub next_quarter_update: EU5Date,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub next_year_update: EU5Date,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub war_direction_years: i32,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub last_warscore_year: i32,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub action: EU5Date,
    #[bin_token("eu5")]
    #[multiple]
    pub battle: Vec<Battle>,

    /// If war goal is superiority
    #[cfg(any())]
    #[bin_token("eu5")]
    pub superiority: Option<SkipValue>,
    /// If war goal is occupy capital
    #[cfg(any())]
    #[bin_token("eu5")]
    pub take_capital: Option<SkipValue>,
    /// If war goal is take province
    #[cfg(any())]
    #[bin_token("eu5")]
    pub take_province: Option<SkipValue>,
    /// If war goal is independence
    #[cfg(any())]
    #[bin_token("eu5")]
    pub independence: Option<SkipValue>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub war_goal_held: i32,

    #[cfg(any())]
    #[bin_token("eu5")]
    pub potential_for_diplomacy: Option<SkipValue>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub scripted_oneway: Option<SkipValue>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub war_reparations: Option<SkipValue>,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub opinion_improvement: Option<SkipValue>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RawWarsEntry {
    None,
    War(RawWar),
}
impl RawWarsEntry {
    pub fn to_war(self) -> Option<RawWar> {
        match self {
            RawWarsEntry::None => None,
            RawWarsEntry::War(war) => Some(war),
        }
    }
    pub fn as_war(&self) -> Option<&RawWar> {
        match self {
            RawWarsEntry::None => None,
            RawWarsEntry::War(war) => Some(war),
        }
    }
}
impl<'de> BinDeserialize<'de> for RawWarsEntry {
    fn take(
        mut stream: BinDeserializer<'de>,
    ) -> ::std::result::Result<(Self, BinDeserializer<'de>), BinError> {
        let peek = stream.peek_token().ok_or(BinError::EOF)?;
        match peek {
            BinToken::ID_OPEN_BRACKET => {
                let (value, rest) = RawWar::take(stream)?;
                return Ok((RawWarsEntry::War(value), rest));
            }
            pdx_parser_macros::eu5_token!("none") => {
                stream.eat_token();
                return Ok((RawWarsEntry::None, stream));
            }
            _ => {
                return Err(BinError::UnexpectedToken(peek));
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct WarParticipant {
    #[bin_token("eu5")]
    pub country: u32,
    #[bin_token("eu5")]
    pub history: WarParticipantHistory,
    /// "Active", "Left"
    #[bin_token("eu5")]
    pub status: Box<str>,
}

/// TODO: what if they rejoin or something?
#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct WarParticipantHistory {
    #[bin_token("eu5")]
    pub request: WarParticipantRequest,
    #[bin_token("eu5")]
    pub joined: Option<WarParticipantJoined>,
    #[bin_token("eu5")]
    pub refused: Option<WarParticipantRefused>,
    #[bin_token("eu5")]
    pub left: Option<WarParticipantLeft>,
}

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct WarParticipantRequest {
    /// Who called them into the war, or `None` if they were the attacker/defender
    #[bin_token("eu5")]
    pub called_ally: Option<u32>,
    /// e.g. "Instigator", "Target", "Subject", "Overlord", "InternationalOrganization", "Scripted"
    #[cfg(any())]
    #[bin_token("eu5")]
    pub reason: Box<str>,
    /// e.g. "Always", "AutoCall", "CanCall"
    #[cfg(any())]
    #[bin_token("eu5")]
    pub join_type: Box<str>,
    /// Seems to be additional info for join_type
    #[cfg(any())]
    #[bin_token("eu5")]
    pub which: Option<Box<str>>,
    #[bin_token("eu5")]
    pub side: WarParticipantSide,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub revolter: Option<Box<str>>,
}

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct WarParticipantJoined {
    #[bin_token("eu5")]
    pub reason: SkipValue,
    #[bin_token("eu5")]
    pub date: EU5Date,
    /// War contribution score, I think
    #[cfg(any())]
    #[bin_token("eu5")]
    pub score: HashMap<String, f64>,
}

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct WarParticipantJoinedLosses {
    /// Keys include "Battle", "Attrition", "Capture"
    #[bin_token("eu5")]
    pub members: Vec<HashMap<String, i32>>,
}

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct WarParticipantRefused {
    #[bin_token("eu5")]
    pub reason: SkipValue,
    #[bin_token("eu5")]
    pub date: EU5Date,
}

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct WarParticipantLeft {
    #[bin_token("eu5")]
    pub reason: SkipValue,
    #[bin_token("eu5")]
    pub date: EU5Date,
}

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub enum WarParticipantSide {
    #[enum_key("Attacker")]
    Attacker,
    #[enum_key("Defender")]
    Defender,
}

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct Battle {
    #[bin_token("eu5")]
    pub location: i32,
    #[bin_token("eu5")]
    pub date: EU5Date,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub result: SkipValue,
    #[bin_token("eu5")]
    pub attacker: BattleParticipation,
    #[bin_token("eu5")]
    pub defender: BattleParticipation,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub war_score: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub war_attacker_win: bool,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub war_score_is_relevant: bool,
}

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct BattleParticipation {
    /// In thousands
    #[bin_token("eu5")]
    pub imprisoned: Vec<f64>,
    /// In thousands
    #[bin_token("eu5")]
    pub losses: Vec<f64>,
    /// In thousands
    #[bin_token("eu5")]
    pub total: Vec<f64>,
    /// Participants on this side of the battle
    #[bin_token("eu5")]
    #[multiple]
    pub who: Vec<BattleWho>,
    /// General/admiral leading the battle for this side
    #[cfg(any())]
    #[bin_token("eu5")]
    pub character: Option<u32>,
    /// In thousands
    #[bin_token("eu5")]
    pub non_levy: f64,
}

#[derive(Debug, Serialize, Deserialize, BinDeserialize)]
pub struct BattleWho {
    /// In thousands
    #[bin_token("eu5")]
    #[default(0.0)]
    pub size: f64,
    /// In thousands
    #[bin_token("eu5")]
    #[default(0.0)]
    pub levy: f64,
    /// In thousands
    #[bin_token("eu5")]
    #[default(0.0)]
    pub mercenary: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub war_exhaustion: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub tradition: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub prestige: f64,
    #[cfg(any())]
    #[bin_token("eu5")]
    pub experience: f64,
    #[bin_token("eu5")]
    pub country: u32,
}
