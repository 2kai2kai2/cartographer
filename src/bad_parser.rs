use serde::{Deserialize, Serialize};
use std::{cmp::min, collections::HashMap, num::ParseFloatError};

use crate::eu4_date::EU4Date;
use anyhow::{anyhow, Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mod {
    Vanilla,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nation {
    pub tag: String,
    pub other_tags: Vec<String>,
    pub development: usize,
    pub prestige: i32,
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

struct NationPartial {
    tag: String,
    other_tags: Vec<String>,
    development: Option<usize>,
    prestige: Option<i32>,
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
    pub attacker_losses: f64,
    pub defender_losses: f64,
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
    pub fn war_scale(&self, player_tags: &Vec<String>) -> f64 {
        let player_attackers = self.player_attackers(player_tags).len();
        let player_defenders = self.player_defenders(player_tags).len();
        let casualties = self.attacker_losses + self.defender_losses;
        if player_attackers <= 1 || player_defenders <= 1 {
            return casualties;
        }

        return casualties * min(player_attackers, player_defenders) as f64;
    }
}

#[derive(Debug, Clone)]
struct WarPartial {
    name: String,
    attackers: Vec<String>,
    defenders: Vec<String>,
    attacker_losses: f64,
    defender_losses: f64,
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
            attacker_losses: 0.0,
            defender_losses: 0.0,
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
                            "prestige" => nation.prestige = Some(line_val.parse::<f64>()? as i32),
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

                    let total: f64 = line
                        .trim()
                        .split(' ')
                        .map(|item| item.parse::<f64>())
                        .collect::<Result<Vec<f64>, ParseFloatError>>()?
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
}
