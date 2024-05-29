use serde::{Deserialize, Serialize};
use std::{cmp::min, collections::HashMap, num::ParseIntError};

use crate::{
    eu4_date::EU4Date,
    raw_parser::{RawEU4Object, RawEU4Scalar, RawEU4Value},
};
use anyhow::{anyhow, Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mod {
    Vanilla,
}

impl<'a> RawEU4Object<'a> {
    pub fn as_color(&self) -> Option<[u8; 3]> {
        return self
            .iter_values()
            .map(|item| {
                item.as_scalar()
                    .and_then(|scalar| Some(scalar.as_int()? as u8))
            })
            .collect::<Option<Vec<_>>>()?
            .try_into()
            .ok();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nation {
    pub tag: String,
    pub other_tags: Vec<String>,
    pub development: usize,
    pub prestige: f64,
    pub stability: i8,
    pub army: f64,
    pub navy: usize,
    pub debt: f64,
    pub treasury: f64,
    pub total_income: f64,
    pub total_expense: f64,
    pub score_place: usize,
    pub capital_id: usize,
    pub overlord: Option<String>,
    pub allies: Vec<String>,
    pub subjects: Vec<String>,
    pub map_color: [u8; 3],
    pub nation_color: [u8; 3],
}
impl Nation {
    pub fn from_parsed_obj(tag: String, obj: &RawEU4Object) -> Result<Nation> {
        let colors = obj
            .get_first_obj("colors")
            .ok_or(anyhow!("Found no colors for a country"))?;
        let map_color: [u8; 3] = colors
            .get_first_obj("map_color")
            .ok_or(anyhow!("no 'map_color' obj"))?
            .as_color()
            .ok_or(anyhow!("Invalid map color length"))?;
        let nation_color: [u8; 3] = colors
            .get_first_obj("country_color")
            .ok_or(anyhow!("no 'country_color' obj"))?
            .as_color()
            .ok_or(anyhow!("Invalid country color length"))?;

        // == FINANCIALS ==
        let treasury = obj
            .get_first_as_float("treasury")
            .ok_or(anyhow!("no float 'treasury'"))?;
        let debt = obj
            .iter_all_KVs()
            .filter_map(|kv| match kv {
                (RawEU4Scalar("loan"), RawEU4Value::Object(loan)) => {
                    loan.get_first_as_float("amount")
                }
                _ => None,
            })
            .sum();
        let total_income = obj
            .get_first_scalar_at_path(["ledger", "lastmonthincome"])
            .and_then(RawEU4Scalar::as_float)
            .unwrap_or(0.0);
        let total_expense = obj
            .get_first_scalar_at_path(["ledger", "lastmonthexpense"])
            .and_then(RawEU4Scalar::as_float)
            .unwrap_or(0.0);

        // == MILITARY ==
        let army: f64 = obj
            .iter_all_KVs()
            .filter_map(|kv| match kv {
                (RawEU4Scalar("army"), RawEU4Value::Object(army_obj)) => Some(army_obj),
                _ => None,
            })
            .flat_map(|army| {
                army.iter_all_KVs().filter_map(|kv| match kv {
                    (RawEU4Scalar("regiment"), RawEU4Value::Object(regiment_obj)) => {
                        Some(regiment_obj.get_first_as_float("strength").unwrap_or(1.0) * 1000.0)
                    }
                    _ => None,
                })
            })
            .sum();
        let navy: usize = obj
            .iter_all_KVs()
            .filter_map(|kv| match kv {
                (RawEU4Scalar("navy"), RawEU4Value::Object(army_obj)) => Some(army_obj),
                _ => None,
            })
            .map(|navy| {
                navy.iter_all_KVs()
                    .filter(|(k, _)| **k == RawEU4Scalar("ship"))
                    .count()
            })
            .sum();

        return Ok(Nation {
            tag,
            other_tags: obj
                .iter_all_KVs()
                .filter_map(|pair| match pair {
                    (RawEU4Scalar("previous_country_tags"), RawEU4Value::Scalar(other_tag)) => {
                        Some(other_tag.as_string())
                    }
                    _ => None,
                })
                .collect(),
            development: obj
                .get_first_as_float("raw_development")
                .unwrap_or_default() as usize,
            prestige: obj
                .get_first_as_float("prestige")
                .ok_or(anyhow!("no float 'prestige"))?,
            stability: obj
                .get_first_as_float("stability")
                .ok_or(anyhow!("no float 'stability'"))? as i8,
            army,
            navy,
            debt,
            treasury,
            total_income,
            total_expense,
            score_place: obj
                .get_first_as_int("score_place")
                .ok_or(anyhow!("No int 'score_place'"))? as usize,
            capital_id: obj
                .get_first_as_int("capital")
                .ok_or(anyhow!("No int 'capital'"))? as usize,
            overlord: obj.get_first_as_string("overlord"),
            allies: obj.get_first_obj("allies").map_or(vec![], |allies| {
                allies
                    .iter_values()
                    .filter_map(RawEU4Value::as_scalar)
                    .map(RawEU4Scalar::as_string)
                    .collect()
            }),
            subjects: obj.get_first_obj("subjects").map_or(vec![], |subjects| {
                subjects
                    .iter_values()
                    .filter_map(RawEU4Value::as_scalar)
                    .map(RawEU4Scalar::as_string)
                    .collect()
            }),
            map_color,
            nation_color,
        });
    }
}

struct NationPartial {
    tag: String,
    other_tags: Vec<String>,
    development: Option<usize>,
    prestige: Option<f64>,
    stability: Option<i8>,
    army: f64,
    navy: usize,
    debt: f64,
    treasury: Option<f64>,
    total_income: Option<f64>,
    total_expense: Option<f64>,
    score_place: Option<usize>,
    capital_id: Option<usize>,
    overlord: Option<String>,
    allies: Vec<String>,
    subjects: Vec<String>,
    map_color: Option<[u8; 3]>,
    nation_color: Option<[u8; 3]>,
}
impl NationPartial {
    fn new(tag: String) -> NationPartial {
        return NationPartial {
            tag,
            other_tags: Vec::new(),
            development: None,
            prestige: None,
            stability: None,
            army: 0.0,
            navy: 1,
            debt: 0.0,
            treasury: None,
            total_income: None,
            total_expense: None,
            score_place: None,
            capital_id: None,
            overlord: None,
            allies: Vec::new(),
            subjects: Vec::new(),
            map_color: None,
            nation_color: None,
        };
    }
}
impl TryFrom<NationPartial> for Nation {
    type Error = anyhow::Error;

    fn try_from(value: NationPartial) -> Result<Self, Self::Error> {
        return Ok(Nation {
            tag: value.tag,
            other_tags: value.other_tags,
            development: value.development.ok_or(Error::msg("missing"))?,
            prestige: value.prestige.ok_or(Error::msg("missing"))?,
            stability: value.stability.ok_or(Error::msg("missing"))?,
            army: value.army,
            navy: value.navy,
            debt: value.debt,
            treasury: value.treasury.ok_or(Error::msg("missing"))?,
            total_income: value.total_income.ok_or(Error::msg("missing"))?,
            total_expense: value.total_expense.ok_or(Error::msg("missing"))?,
            score_place: value.score_place.ok_or(Error::msg("missing"))?,
            capital_id: value.capital_id.ok_or(Error::msg("missing"))?,
            overlord: value.overlord,
            allies: value.allies,
            subjects: value.subjects,
            map_color: value.map_color.ok_or(Error::msg("missing"))?,
            nation_color: value.nation_color.ok_or(Error::msg("missing"))?,
        });
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WarResult {
    WhitePeace = 1,
    AttackerVictory = 2,
    DefenderVictory = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct War {
    pub name: String,
    pub attackers: Vec<String>,
    pub defenders: Vec<String>,
    pub attacker_losses: i64,
    pub defender_losses: i64,
    pub start_date: EU4Date,
    pub end_date: Option<EU4Date>,
    pub result: Option<WarResult>,
}

impl War {
    pub fn player_attackers(&self, player_tags: &Vec<String>) -> Vec<String> {
        return self
            .attackers
            .iter()
            .filter(|a| player_tags.contains(a))
            .cloned()
            .collect();
    }
    pub fn player_defenders(&self, player_tags: &Vec<String>) -> Vec<String> {
        return self
            .defenders
            .iter()
            .filter(|d| player_tags.contains(d))
            .cloned()
            .collect();
    }

    /** An evaluation of how 'significant' the war probably is  */
    pub fn war_scale(&self, player_tags: &Vec<String>) -> i64 {
        let player_attackers = self.player_attackers(player_tags).len();
        let player_defenders = self.player_defenders(player_tags).len();
        let casualties = self.attacker_losses + self.defender_losses;
        if player_attackers <= 1 || player_defenders <= 1 {
            return casualties;
        }

        return casualties * min(player_attackers, player_defenders) as i64;
    }

    pub fn is_player_war(&self, player_tags: &Vec<String>) -> bool {
        return self.attackers.iter().any(|a| player_tags.contains(a))
            && self.defenders.iter().any(|d| player_tags.contains(d));
    }

    /// There are nonexistant wars in save files, so keep an option
    pub fn from_parsed_obj(obj: &RawEU4Object) -> Result<Option<War>> {
        let mut attackers: Vec<String> = Vec::new();
        let mut defenders: Vec<String> = Vec::new();
        let mut earliest_date: Option<EU4Date> = None;
        let mut latest_date: Option<EU4Date> = None;
        for (date, value) in obj
            .get_first_obj("history")
            .ok_or(anyhow!("No history in war"))?
            .iter_all_KVs()
            .filter_map(|(k, v)| {
                Some((
                    k.as_date()?,
                    v.as_object().expect("date events should be objects"),
                ))
            })
        {
            for (event, value) in value.iter_all_KVs() {
                match (event.0, value) {
                    ("add_attacker", RawEU4Value::Scalar(value)) => {
                        attackers.push(value.as_string());
                        match earliest_date {
                            None => earliest_date = Some(date),
                            Some(prev_date) if date < prev_date => earliest_date = Some(date),
                            _ => {}
                        }
                    }
                    ("add_defender", RawEU4Value::Scalar(value)) => {
                        defenders.push(value.as_string());
                        match earliest_date {
                            None => earliest_date = Some(date),
                            Some(prev_date) if date < prev_date => earliest_date = Some(date),
                            _ => {}
                        }
                    }
                    ("rem_attacker", RawEU4Value::Scalar(_))
                    | ("rem_defender", RawEU4Value::Scalar(_)) => match latest_date {
                        None => latest_date = Some(date),
                        Some(prev_date) if prev_date < date => latest_date = Some(date),
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        let Some(start_date) = earliest_date else {
            return Ok(None);
        };

        let mut attacker_losses: i64 = 0;
        let mut defender_losses: i64 = 0;
        for (key, value) in obj.iter_all_KVs() {
            if key.0 != "participants" {
                continue;
            }
            let RawEU4Value::Object(obj) = value else {
                // this shouldn't happen?
                continue;
            };

            let Some(tag) = obj.get_first_scalar("tag") else {
                continue;
            };
            let Some(losses) = obj.get_first_object_at_path(["losses", "members"]) else {
                continue;
            };
            let losses: i64 = losses
                .iter_values()
                .filter_map(RawEU4Value::as_scalar)
                .filter_map(RawEU4Scalar::as_int)
                .sum();
            if attackers.contains(&tag.as_string()) {
                attacker_losses += losses;
            } else if defenders.contains(&tag.as_string()) {
                defender_losses += losses;
            }
        }

        let name = obj
            .get_first_scalar("name")
            .ok_or(anyhow!("No name in war"))?
            .as_string();
        return Ok(Some(War {
            name: name.clone(),
            attackers: attackers.clone(),
            defenders: defenders.clone(),
            attacker_losses,
            defender_losses,
            start_date,
            end_date: latest_date,
            result: match obj.get_first_scalar("outcome") {
                Some(RawEU4Scalar("1")) => Some(WarResult::WhitePeace),
                Some(RawEU4Scalar("2")) => Some(WarResult::AttackerVictory),
                Some(RawEU4Scalar("3")) => Some(WarResult::DefenderVictory),
                _ => None,
            },
        }));
    }
}

#[derive(Debug, Clone)]
struct WarPartial {
    name: String,
    attackers: Vec<String>,
    defenders: Vec<String>,
    attacker_losses: i64,
    defender_losses: i64,
    start_date: Option<EU4Date>,
    end_date: Option<EU4Date>,
    result: Option<WarResult>,
}
impl WarPartial {
    fn new(name: String) -> WarPartial {
        return WarPartial {
            name,
            attackers: Vec::new(),
            defenders: Vec::new(),
            attacker_losses: 0,
            defender_losses: 0,
            start_date: None,
            end_date: None,
            result: None,
        };
    }

    fn is_player_war(&self, player_tags: &Vec<String>) -> bool {
        return self.attackers.iter().any(|a| player_tags.contains(a))
            && self.defenders.iter().any(|d| player_tags.contains(d));
    }
}

impl TryFrom<WarPartial> for War {
    type Error = anyhow::Error;

    fn try_from(value: WarPartial) -> Result<Self, Self::Error> {
        return Ok(War {
            name: value.name,
            attackers: value.attackers,
            defenders: value.defenders,
            attacker_losses: value.attacker_losses,
            defender_losses: value.defender_losses,
            start_date: value
                .start_date
                .ok_or(Error::msg("Partial war data is missing start date"))?,
            end_date: value.end_date,
            result: value.result,
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveGame {
    pub all_nations: HashMap<String, Nation>,
    /** tag: playername */
    pub player_tags: HashMap<String, String>,
    pub provinces: HashMap<u64, String>,
    pub dlc: Vec<String>,
    pub great_powers: Vec<String>,
    pub date: EU4Date,
    pub multiplayer: bool,
    pub age: Option<String>,
    pub hre: Option<String>,
    pub china: Option<String>,
    pub crusade: Option<String>,
    pub player_wars: Vec<War>,
    pub game_mod: Mod,
}

impl SaveGame {
    pub fn player_nations(&self) -> impl Iterator<Item = (&String, &Nation)> {
        return self
            .player_tags
            .iter()
            .filter_map(|(tag, player)| Some((player, self.all_nations.get(tag)?)));
    }

    /** Gets the player of a nation, including former tags */
    pub fn tag_player(&self, tag: &String) -> Option<&String> {
        return self.player_tags.get(tag).or_else(|| {
            for (player, nation) in self.player_nations() {
                if nation.other_tags.contains(tag) {
                    return Some(player);
                }
            }
            return None;
        });
    }

    pub fn bad_parser(file_text: &str) -> Result<SaveGame> {
        let mut nations: HashMap<String, NationPartial> = HashMap::new();
        let mut player_tags: HashMap<String, String> = HashMap::new();
        let mut provinces: HashMap<u64, String> = HashMap::new();
        let mut dlc: Vec<String> = Vec::new();
        let mut great_powers: Vec<String> = Vec::new();
        let mut date: Option<EU4Date> = None;
        let mut age: Option<String> = None;
        let mut multiplayer: Option<bool> = None;
        let mut hre: Option<String> = None;
        let mut china: Option<String> = None;
        let mut crusade: Option<String> = None;
        let mut player_wars: Vec<War> = Vec::new();
        let mut game_mod: Mod = Mod::Vanilla;

        let mut brackets: Vec<&str> = Vec::new();
        let mut current_read_war: Option<WarPartial> = None;
        let mut current_read_war_tag: Option<&str> = None;
        let mut current_read_war_last_leave: Option<EU4Date> = None;
        let mut last_player_in_list: Option<&str> = None;

        for (line_num, line) in file_text.lines().enumerate() {
            if line.contains('{') {
                if line.contains('}') {
                    continue;
                }
                let open_count = line.matches('{').count();
                if open_count == 1 {
                    brackets.push(line.trim().trim_end_matches('{').trim_end_matches('='));
                } else {
                    for _ in 0..open_count {
                        brackets.push("{");
                    }
                }
                continue;
            }

            if line.contains("}") {
                brackets.pop().ok_or(Error::msg("Brackets pop"))?;
                continue;
            }

            // Otherwise, it's just a normal line
            let equals_index = line.find('=');
            let quote_index = line.find('"');
            let line_key: Option<&str>;
            let line_val: &str;
            if equals_index.is_some_and(|e| !(quote_index.is_some_and(|q| q < e))) {
                let (k, v) = line.split_once('=').ok_or(Error::msg("LogicError"))?;
                line_key = Some(k.trim());
                line_val = v.trim();
            } else {
                line_key = None;
                line_val = line.trim();
            }

            // This is where we get to do stuff
            if brackets.is_empty() {
                match line_key {
                    Some("date") => date = Some(line_val.parse::<EU4Date>()?),
                    Some("multi_player") => multiplayer = Some(line_val == "yes"),
                    Some("current_age") => age = Some(line_val.trim_matches('"').into()),
                    _ => {}
                }
                continue;
            }

            match (brackets.as_slice(), line_key) {
                (["players_countries"], None) => {
                    if last_player_in_list == None {
                        last_player_in_list = Some(
                            line_val
                                .trim()
                                .strip_prefix('"')
                                .and_then(|p| p.strip_suffix('"'))
                                .ok_or(anyhow!("Player name was not quoted"))?,
                        );
                    } else {
                        player_tags.insert(
                            line_val.trim_matches('"').into(),
                            last_player_in_list.ok_or(Error::msg("PlayerTags"))?.into(),
                        );
                        last_player_in_list = None;
                    }
                }
                // TODO: mods clause
                (["great_powers", "original"], Some("country")) => {
                    if great_powers.len() < 8 {
                        great_powers.push(line_val.trim_matches('"').into());
                    }
                }
                (["empire"], Some("emperor")) => {
                    hre = Some(line_val.trim_matches('"').into());
                }
                (["celestial_empire"], Some("emperor")) => {
                    china = Some(line_val.trim_matches('"').into());
                }
                (["religion_instance_data", "catholic", "papacy"], Some("crusade_target")) => {
                    crusade = Some(line_val.trim_matches('"').into());
                }
                // TODO: papal controller
                (["provinces", province], Some("owner")) => {
                    let province_id = province
                        .get(1..)
                        .ok_or(Error::msg("ProvinceReadError"))?
                        .parse::<u64>()
                        .or(Err(Error::msg("ProvinceReadError")))?;
                    let first_quote = line.find('"').ok_or(Error::msg("ProvinceReadError"))?;
                    let last_quote = line.rfind('"').ok_or(Error::msg("ProvinceReadError"))?;
                    provinces.insert(
                        province_id,
                        line.get((first_quote + 1)..last_quote)
                            .ok_or(Error::msg("ProvinceReadError"))?
                            .into(),
                    );
                }

                // === READ COUNTRIES ===
                (["countries", tag], Some("government_rank")) => {
                    nations.insert(tag.to_string(), NationPartial::new(tag.to_string()));
                }

                (["countries", tag], Some(country_key)) => {
                    let nation = nations.get_mut(*tag);
                    if let Some(nation) = nation {
                        match country_key {
                            "previous_country_tags" => {
                                nation
                                    .other_tags
                                    .push(line_val.trim().trim_matches('"').to_string());
                            }
                            "raw_development" => {
                                nation.development = Some(line_val.parse::<f64>()? as usize)
                            }
                            "capital" => nation.capital_id = Some(line_val.parse()?),
                            "score_place" => {
                                nation.score_place = Some(line_val.parse::<f64>()? as usize)
                            }
                            "prestige" => nation.prestige = Some(line_val.parse::<f64>()?),
                            "stability" => nation.stability = Some(line_val.parse::<f64>()? as i8),
                            "treasury" => nation.treasury = Some(line_val.parse::<f64>()?),
                            "overlord" => nation.overlord = Some(line_val.trim_matches('"').into()),
                            _ => {}
                        }
                    }
                }
                (["countries", tag, "loan"], Some("amount")) => {
                    let nation = nations.get_mut(*tag).ok_or(Error::msg("OutOfOrder loan"))?;
                    nation.debt += line_val.parse::<f64>()?;
                }
                (["countries", tag, "ledger"], Some("lastmonthincome")) => {
                    let nation = nations
                        .get_mut(*tag)
                        .ok_or(Error::msg("OutOfOrder income"))?;
                    nation.total_income = Some(line_val.parse::<f64>()?);
                }
                (["countries", tag, "ledger"], Some("lastmonthexpense")) => {
                    let nation = nations
                        .get_mut(*tag)
                        .ok_or(Error::msg("OutOfOrder expense"))?;
                    nation.total_expense = Some(line_val.parse::<f64>()?);
                }
                (["countries", tag, "subjects"], None) => {
                    let nation = nations
                        .get_mut(*tag)
                        .ok_or(Error::msg("OutOfOrder subjects"))?;
                    nation.subjects.extend(
                        line_val
                            .to_string()
                            .split(char::is_whitespace)
                            .map(|sub| sub.to_string()),
                    );
                }
                (["countries", tag, "allies"], None) => {
                    let nation = nations
                        .get_mut(*tag)
                        .ok_or(Error::msg("OutOfOrder allies"))?;
                    nation.allies.extend(
                        line_val
                            .to_string()
                            .split(char::is_whitespace)
                            .map(|ally| ally.to_string()),
                    );
                }
                (["countries", tag, "army", "regiment"], Some("morale")) => {
                    let nation = nations
                        .get_mut(*tag)
                        .ok_or(Error::msg("OutOfOrder morale"))?;
                    nation.army += 1000.0;
                }
                (["countries", tag, "army", "regiment"], Some("strength")) => {
                    let nation = nations
                        .get_mut(*tag)
                        .ok_or(Error::msg("OutOfOrder strength"))?;
                    // ignore errors here??
                    let proportion = line_val.parse::<f64>();
                    if let Ok(proportion) = proportion {
                        nation.army -= 1000.0 * (1.0 - proportion);
                    }
                }
                (["countries", tag, "navy", "ship"], Some("home")) => {
                    let nation = nations.get_mut(*tag).ok_or(Error::msg("OutOfOrder navy"))?;
                    nation.navy += 1;
                }
                (["countries", tag, "colors", "map_color"], None) => {
                    let nation = nations.get_mut(*tag).ok_or(Error::msg("OutOfOrder map"))?;
                    let parts: Vec<&str> = line_val.trim().split(' ').collect();
                    if let [r, g, b] = parts.as_slice() {
                        nation.map_color = Some([r.parse()?, g.parse()?, b.parse()?]);
                    }
                }
                (["countries", tag, "colors", "country_color"], None) => {
                    let nation = nations
                        .get_mut(*tag)
                        .ok_or(Error::msg("OutOfOrder country"))?;
                    let parts: Vec<&str> = line_val.trim().split(' ').collect();
                    if let [r, g, b] = parts.as_slice() {
                        nation.nation_color = Some([r.parse()?, g.parse()?, b.parse()?]);
                    }
                }

                // === READ WARS ===
                (["previous_war"], Some("name")) => {
                    if let Some(current_war) = &mut current_read_war {
                        if current_war.is_player_war(&player_tags.keys().cloned().collect()) {
                            current_war.end_date = current_read_war_last_leave;
                            player_wars.push(current_war.clone().try_into()?);
                        }
                    }
                    current_read_war =
                        Some(WarPartial::new(line_val.trim_matches('"').to_string()));
                }
                (["previous_war", "history", raw_date], Some(event)) => {
                    let war = current_read_war
                        .as_mut()
                        .ok_or(Error::msg("OutOfOrder war history"))?;
                    let Ok(date) = raw_date.parse::<EU4Date>() else {
                        // ignore non-date history
                        continue;
                    };
                    let tag = line_val.trim_matches('"').to_string();
                    match event {
                        "add_attacker" => {
                            war.attackers.push(tag);
                            if let None = war.start_date {
                                war.start_date = Some(date);
                            }
                        }
                        "add_defender" => war.defenders.push(tag),
                        "rem_attacker" | "rem_defender" => current_read_war_last_leave = Some(date),
                        _ => {}
                    }
                }
                (["previous_war", "participants"], Some("tag")) => {
                    current_read_war_tag = Some(line_val.trim_matches('"'));
                }
                (["previous_war", "participants", "losses", "members"], None) => {
                    let war = current_read_war
                        .as_mut()
                        .ok_or(Error::msg("OutOfOrder losses1"))?;
                    let tag = current_read_war_tag.ok_or(Error::msg("OutOfOrder losses2"))?;

                    let total: i64 = line
                        .trim()
                        .split(' ')
                        .map(|item| item.parse::<i64>())
                        .collect::<Result<Vec<i64>, ParseIntError>>()?
                        .iter()
                        .sum();

                    if war.attackers.contains(&tag.to_string()) {
                        war.attacker_losses += total;
                    } else if war.defenders.contains(&tag.to_string()) {
                        war.defender_losses += total;
                    } else {
                        println!(
                            "oopsies, something went wrong with attacker/defender losses line {:?} {} {}",
                            current_read_war_tag,
                            line_num,
                            line
                        );
                    }
                }
                (["previous_war"], Some("outcome")) => {
                    let war = current_read_war
                        .as_mut()
                        .ok_or(Error::msg("OutOfOrder outcome"))?;
                    war.result = match line_val.trim() {
                        "1" => Some(WarResult::WhitePeace),
                        "2" => Some(WarResult::AttackerVictory),
                        "3" => Some(WarResult::DefenderVictory),
                        _ => {
                            println!("Unknown war outcome at line {}", line_num);
                            None
                        }
                    };

                    if war.is_player_war(&player_tags.keys().cloned().collect()) {
                        war.end_date = current_read_war_last_leave;
                        player_wars.push(war.clone().try_into()?);
                        current_read_war = None;
                    }
                }

                _ => {}
            }
        }

        return Ok(SaveGame {
            // we remove any nations that don't have all their data (they probably don't exist)
            all_nations: nations
                .into_iter()
                .filter_map(|(tag, nation)| Some((tag, nation.try_into().ok()?)))
                .collect::<HashMap<String, Nation>>(),
            player_tags,
            provinces,
            dlc,
            great_powers,
            date: date.ok_or(Error::msg("Date was never found"))?,
            multiplayer: multiplayer.ok_or(Error::msg("Multiplayer status was never found"))?,
            age,
            hre,
            china,
            crusade,
            player_wars,
            game_mod,
        });
    }

    pub fn new_parser(file_text: &str) -> Option<SaveGame> {
        let (_, obj) = RawEU4Object::parse_object_inner(file_text).unwrap();

        // TODO
        let all_nations = obj
            .get_first_obj("countries")
            .unwrap()
            .iter_all_KVs()
            .filter_map(|kv| match kv {
                (RawEU4Scalar(tag), RawEU4Value::Object(nation)) => Some((
                    tag.to_string(),
                    Nation::from_parsed_obj(tag.to_string(), nation).unwrap(),
                )),
                _ => None,
            })
            .collect();
        let player_tags: Vec<&RawEU4Scalar> = obj
            .get_first_obj("players_countries")?
            .iter_values()
            .map(RawEU4Value::as_scalar)
            .collect::<Option<Vec<_>>>()
            .unwrap();
        let player_tags: HashMap<String, String> = player_tags
            .chunks_exact(2)
            .map(|v| match v {
                [player, tag] => Some((tag.as_string(), player.as_string())),
                _ => None,
            })
            .collect::<Option<HashMap<_, _>>>()
            .unwrap();
        let provinces: HashMap<u64, String> = obj
            .get_first_obj("provinces")?
            .iter_all_KVs()
            .filter_map(|(k, v)| Some((k, v.as_object()?)))
            .filter_map(|(k, v)| {
                Some((
                    k.as_int()?.abs() as u64,
                    v.get_first_scalar("owner")?.as_string(),
                ))
            })
            .collect();
        let dlc: Vec<String> = obj
            .get_first_obj("dlc_enabled")?
            .iter_values()
            .filter_map(|v| match v {
                RawEU4Value::Scalar(scalar) => Some(scalar.as_string()),
                _ => None,
            })
            .collect();
        let great_powers = Vec::new();
        let date = obj.get_first_scalar("date");

        return Some(SaveGame {
            all_nations,
            player_tags,
            provinces,
            dlc,
            great_powers,
            date: date.unwrap().as_date().unwrap(),
            multiplayer: obj
                .get_first_scalar("multi_player")
                .unwrap()
                .as_bool()
                .unwrap(),
            age: obj
                .get_first_scalar("current_age")
                .map(RawEU4Scalar::as_string),
            hre: obj
                .get_first_scalar_at_path(["empire", "emperor"])
                .map(RawEU4Scalar::as_string),
            china: obj
                .get_first_scalar_at_path(["celestial_empire", "emperor"])
                .map(RawEU4Scalar::as_string),
            crusade: None,
            player_wars: obj
                .iter_all_KVs()
                .filter(|(k, _)| k.0 == "active_war" || k.0 == "previous_war")
                .filter_map(|(_, v)| match v {
                    RawEU4Value::Object(o) => Some(o),
                    _ => None,
                })
                .map(War::from_parsed_obj)
                .collect::<Result<Vec<_>>>()
                .expect("oh no invalid wars?")
                .into_iter()
                .filter_map(|a| a)
                .collect(),
            game_mod: Mod::Vanilla,
        });
    }
}
